use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use hidapi::{DeviceInfo, HidApi, HidDevice};

pub const VENDOR_ID: u16 = 0x6993;
pub const PRODUCT_ID: u16 = 0xb019;
const CONTROL_USAGE_PAGE: u16 = 0x000b;
const CONTROL_USAGE: u16 = 0x0005;
const REPORT_ID: u8 = 0xc8;
const REPORT_LEN: usize = 64;
const CAMERA_SET: u16 = 0x0211;
const CAMERA_GET: u16 = 0x0212;
const CAMERA_SAVE: u32 = 46;
const CAMERA_FOV: u32 = 56;
const CAMERA_PARAM_LEN: usize = 32;
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);
const APPLY_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fov {
    Deg70,
    Deg90,
    Deg120,
}

impl Fov {
    pub fn degrees(self) -> i32 {
        match self {
            Self::Deg70 => 70,
            Self::Deg90 => 90,
            Self::Deg120 => 120,
        }
    }
}

impl TryFrom<i32> for Fov {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            70 => Ok(Self::Deg70),
            90 => Ok(Self::Deg90),
            120 => Ok(Self::Deg120),
            _ => bail!("unsupported FOV {value}; choose 70, 90, or 120"),
        }
    }
}

pub struct Camera {
    device: HidDevice,
    next_transaction: u16,
}

impl Camera {
    pub fn open(serial: Option<&str>) -> Result<Self> {
        let api = HidApi::new().context("could not initialize HID access")?;
        let info = select_device(&api, serial)?;
        let device = api
            .open_path(info.path())
            .context("could not open the Yealink control interface")?;

        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u16;

        Ok(Self {
            device,
            next_transaction: seed,
        })
    }

    pub fn set_fov(&mut self, fov: Fov) -> Result<()> {
        self.exchange_camera(CAMERA_SET, CAMERA_FOV, fov.degrees())?;
        let deadline = Instant::now() + APPLY_TIMEOUT;
        loop {
            let actual = self.get_fov()?;
            if actual == fov {
                self.exchange_camera(CAMERA_SET, CAMERA_SAVE, 1)?;
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "camera reported {} degrees after setting {}; close applications that are capturing video from it, such as OBS",
                    actual.degrees(),
                    fov.degrees()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn get_fov(&mut self) -> Result<Fov> {
        let reply = self.exchange_camera(CAMERA_GET, CAMERA_FOV, -1)?;
        if reply.payload.len() < 8 {
            bail!("camera returned a truncated FOV response");
        }
        let operation = u32::from_le_bytes(reply.payload[0..4].try_into().unwrap());
        if operation != CAMERA_FOV {
            bail!("camera returned operation {operation}, expected {CAMERA_FOV}");
        }
        let degrees = i32::from_le_bytes(reply.payload[4..8].try_into().unwrap());
        match degrees {
            70 => Ok(Fov::Deg70),
            90 => Ok(Fov::Deg90),
            120 => Ok(Fov::Deg120),
            _ => bail!("camera returned unsupported FOV {degrees}"),
        }
    }

    fn exchange_camera(&mut self, message: u16, operation: u32, value: i32) -> Result<Reply> {
        let mut payload = [0_u8; CAMERA_PARAM_LEN];
        payload[0..4].copy_from_slice(&operation.to_le_bytes());
        payload[4..8].copy_from_slice(&value.to_le_bytes());
        self.exchange(2, message, &payload)
    }

    fn exchange(&mut self, protocol: u8, message: u16, payload: &[u8]) -> Result<Reply> {
        let transaction = self.next_transaction;
        self.next_transaction = self.next_transaction.wrapping_add(1);
        let report = rc8_report(protocol, message, transaction, payload);
        let written = self.device.write(&report).context("HID write failed")?;
        if written != report.len() {
            bail!("short HID write: wrote {written} of {} bytes", report.len());
        }
        self.read_reply(protocol, message, transaction)
    }

    fn read_reply(&self, protocol: u8, message: u16, transaction: u16) -> Result<Reply> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        let mut report = [0_u8; REPORT_LEN];

        while Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let timeout_ms = remaining.as_millis().min(i32::MAX as u128) as i32;
            let size = self
                .device
                .read_timeout(&mut report, timeout_ms.max(1))
                .context("HID read failed")?;
            if size == 0 {
                break;
            }
            let Some(frame) = parse_frame(&report[..size])? else {
                continue;
            };
            if frame.fragment != 0x03 {
                continue;
            }
            let reply = parse_message(frame.protocol, &frame.data)?;
            if reply.protocol == protocol
                && reply.message == message
                && reply.transaction == transaction
            {
                if reply.status < 0 {
                    bail!("camera rejected the command with status {}", reply.status);
                }
                return Ok(reply);
            }
        }

        bail!("timed out waiting for a reply from the camera")
    }
}

fn select_device<'a>(api: &'a HidApi, serial: Option<&str>) -> Result<&'a DeviceInfo> {
    let candidates: Vec<_> = api
        .device_list()
        .filter(|info| {
            info.vendor_id() == VENDOR_ID
                && info.product_id() == PRODUCT_ID
                && info.usage_page() == CONTROL_USAGE_PAGE
                && info.usage() == CONTROL_USAGE
                && serial.is_none_or(|wanted| info.serial_number() == Some(wanted))
        })
        .collect();

    match candidates.as_slice() {
        [] => bail!(
            "Yealink UVC30 control interface not found; connect the camera and close Yealink USB Connect"
        ),
        [info] => Ok(info),
        _ if candidates.iter().any(|info| info.serial_number().is_none()) => bail!(
            "multiple Yealink UVC30 cameras found and at least one has no USB serial number; disconnect the others"
        ),
        _ => bail!("multiple Yealink UVC30 cameras found; select one with --serial"),
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Reply {
    protocol: u8,
    message: u16,
    transaction: u16,
    status: i32,
    payload: Vec<u8>,
}

struct Frame {
    protocol: u8,
    fragment: u8,
    data: Vec<u8>,
}

fn rc8_report(protocol: u8, message: u16, transaction: u16, payload: &[u8]) -> [u8; REPORT_LEN] {
    assert!(payload.len() <= REPORT_LEN - 20);
    let mut report = [0_u8; REPORT_LEN];
    report[0] = REPORT_ID;
    report[1] = (protocol << 3) | 0x03;
    report[2] = (16 + payload.len()) as u8;
    report[4..6].copy_from_slice(&message.to_le_bytes());
    report[6..8].copy_from_slice(&transaction.to_le_bytes());
    report[8..10].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    report[20..20 + payload.len()].copy_from_slice(payload);
    report
}

#[cfg(test)]
fn camera_report(message: u16, transaction: u16, value: i32) -> [u8; REPORT_LEN] {
    camera_param_report(message, transaction, CAMERA_FOV, value)
}

#[cfg(test)]
fn camera_param_report(
    message: u16,
    transaction: u16,
    operation: u32,
    value: i32,
) -> [u8; REPORT_LEN] {
    let mut payload = [0_u8; CAMERA_PARAM_LEN];
    payload[0..4].copy_from_slice(&operation.to_le_bytes());
    payload[4..8].copy_from_slice(&value.to_le_bytes());
    rc8_report(2, message, transaction, &payload)
}

#[cfg(test)]
fn parse_reply(report: &[u8]) -> Result<Option<Reply>> {
    let Some(frame) = parse_frame(report)? else {
        return Ok(None);
    };
    if frame.fragment != 0x03 {
        return Ok(None);
    }
    Ok(Some(parse_message(frame.protocol, &frame.data)?))
}

fn parse_frame(report: &[u8]) -> Result<Option<Frame>> {
    if report.first() != Some(&REPORT_ID) {
        return Ok(None);
    }
    let control_byte = *report
        .get(1)
        .ok_or_else(|| anyhow!("truncated HID report"))?;
    let logical_len = *report
        .get(2)
        .ok_or_else(|| anyhow!("truncated HID report"))? as usize;
    let data_start = 4;
    let data_end = data_start + logical_len;
    let data = report
        .get(data_start..data_end)
        .ok_or_else(|| anyhow!("invalid HID report length {logical_len}"))?;
    Ok(Some(Frame {
        protocol: control_byte >> 3,
        fragment: control_byte & 0x07,
        data: data.to_vec(),
    }))
}

fn parse_message(protocol: u8, data: &[u8]) -> Result<Reply> {
    if data.len() < 16 {
        bail!("truncated RC8 response");
    }

    let payload_len = u16::from_le_bytes(data[4..6].try_into().unwrap()) as usize;
    let payload_end = 16 + payload_len;
    let payload = data
        .get(16..payload_end)
        .ok_or_else(|| anyhow!("truncated RC8 response payload"))?;

    Ok(Reply {
        protocol,
        message: u16::from_le_bytes(data[0..2].try_into().unwrap()),
        transaction: u16::from_le_bytes(data[2..4].try_into().unwrap()),
        status: i32::from_le_bytes(data[8..12].try_into().unwrap()),
        payload: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_observed_camera_get_report() {
        let report = camera_report(CAMERA_GET, 0x1234, -1);

        assert_eq!(
            &report[..28],
            &[
                0xc8, 0x13, 0x30, 0x00, 0x12, 0x02, 0x34, 0x12, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x38, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            ]
        );
        assert!(report[28..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn builds_the_camera_set_report() {
        let report = camera_report(CAMERA_SET, 7, 90);

        assert_eq!(&report[4..8], &[0x11, 0x02, 0x07, 0x00]);
        assert_eq!(&report[20..28], &[0x38, 0, 0, 0, 0x5a, 0, 0, 0]);
    }

    #[test]
    fn builds_the_camera_parameter_save_report() {
        let report = camera_param_report(CAMERA_SET, 8, CAMERA_SAVE, 1);

        assert_eq!(&report[4..8], &[0x11, 0x02, 0x08, 0x00]);
        assert_eq!(&report[20..28], &[0x2e, 0, 0, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn parses_a_camera_reply() {
        let mut report = camera_report(CAMERA_GET, 0x1234, 120);
        report[12..16].copy_from_slice(&0_i32.to_le_bytes());

        let reply = parse_reply(&report).unwrap().unwrap();

        assert_eq!(reply.message, CAMERA_GET);
        assert_eq!(reply.transaction, 0x1234);
        assert_eq!(reply.status, 0);
        assert_eq!(&reply.payload[..8], &[0x38, 0, 0, 0, 0x78, 0, 0, 0]);
    }

    #[test]
    fn ignores_unrelated_hid_reports() {
        assert_eq!(parse_reply(&[0x02, 0x40, 0, 0]).unwrap(), None);
    }

    #[test]
    fn honors_the_declared_response_payload_length() {
        let mut report = camera_report(CAMERA_GET, 1, 120);
        report[8..10].copy_from_slice(&4_u16.to_le_bytes());

        let reply = parse_reply(&report).unwrap().unwrap();

        assert_eq!(reply.payload, &[0x38, 0, 0, 0]);
    }
}
