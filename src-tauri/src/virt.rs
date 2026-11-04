//! Virtual Xbox 360 and DualShock 4 targets on ViGEmBus.

use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use vigem_client::{Client, DS4Report, DualShock4Wired, TargetId, XGamepad, Xbox360Wired};

use crate::settings::OutputMode;

pub const NO_LED: u8 = 0xFF;

pub struct XboxVirt {
    pub target: Xbox360Wired<Client>,
    pub last: Option<XGamepad>,
    pub slot: Option<u32>,
    pub led: Arc<AtomicU8>,
    _notify: Option<JoinHandle<()>>,
}

pub struct Ds4Virt {
    pub target: DualShock4Wired<Client>,
    pub last: Option<DS4Report>,
}

pub enum Virt {
    Xbox(XboxVirt),
    Ds4(Ds4Virt),
}

impl Virt {
    pub fn is_xbox(&self) -> bool {
        matches!(self, Self::Xbox(_))
    }
}

pub fn plug(mode: OutputMode, rumble: Arc<AtomicU16>) -> Result<Virt, String> {
    match mode {
        OutputMode::Xbox360 => plug_xbox(rumble).map(Virt::Xbox),
        OutputMode::DualShock4 => plug_ds4().map(Virt::Ds4),
        OutputMode::Native => Err("el modo nativo no crea un control virtual".into()),
    }
}

pub fn plug_ds4() -> Result<Ds4Virt, String> {
    let client = Client::connect().map_err(|_| "ViGEmBus no está instalado".to_string())?;
    let mut target = DualShock4Wired::new(client, TargetId::DUALSHOCK4_WIRED);
    target.plugin().map_err(|e| format!("no se pudo crear el DualShock 4 virtual: {e:?}"))?;
    target.wait_ready().map_err(|e| format!("el DualShock 4 virtual no respondió: {e:?}"))?;
    Ok(Ds4Virt { target, last: None })
}

fn plug_xbox(rumble: Arc<AtomicU16>) -> Result<XboxVirt, String> {
    let client = Client::connect().map_err(|_| "ViGEmBus no está instalado".to_string())?;
    let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
    target.plugin().map_err(|e| format!("no se pudo crear el control virtual: {e:?}"))?;
    target.wait_ready().map_err(|e| format!("el control virtual no respondió: {e:?}"))?;
    let led = Arc::new(AtomicU8::new(NO_LED));
    let led2 = led.clone();
    let notify = target.request_notification().ok().map(|n| {
        n.spawn_thread(move |_, data| {
            rumble.store(((data.large_motor as u16) << 8) | data.small_motor as u16, Ordering::Relaxed);
            led2.store(data.led_number, Ordering::Relaxed);
        })
    });
    Ok(XboxVirt { target, last: None, slot: None, led, _notify: notify })
}
