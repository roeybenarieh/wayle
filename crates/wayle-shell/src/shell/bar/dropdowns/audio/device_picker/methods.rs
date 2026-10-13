use std::sync::Arc;

use relm4::prelude::*;
use tracing::warn;
use wayle_audio::{
    AudioService,
    core::device::{input::InputDevice, output::OutputDevice},
    types::device::DevicePort,
};

use super::{
    DevicePicker,
    messages::{DeviceInfo, DevicePickerOutput},
};
use crate::shell::bar::dropdowns::audio::{VolumeSectionKind, helpers};

impl DevicePicker {
    pub(super) fn apply_device_list(&mut self, list: Vec<DeviceInfo>) {
        let mut guard = self.devices.guard();
        guard.clear();
        for info in list {
            guard.push_back(info);
        }
    }

    pub(super) fn select_device(&self, index: usize, sender: &ComponentSender<Self>) {
        let _ = sender.output(DevicePickerOutput::NavigateBack);

        match self.kind {
            VolumeSectionKind::Output => {
                let Some(device) = self.cached_output_devices.get(index).cloned() else {
                    return;
                };
                sender.command(|_out, _shutdown| async move {
                    if let Err(err) = device.set_as_default().await {
                        warn!(error = %err, "failed to set default output");
                    }
                });
            }
            VolumeSectionKind::Input => {
                let mut physical_inputs = self
                    .cached_input_devices
                    .iter()
                    .filter(|device| !device.is_monitor.get());
                let Some(device) = physical_inputs.nth(index).cloned() else {
                    return;
                };
                sender.command(|_out, _shutdown| async move {
                    if let Err(err) = device.set_as_default().await {
                        warn!(error = %err, "failed to set default input");
                    }
                });
            }
        }
    }

    pub(super) fn build_device_list(
        audio: &AudioService,
        kind: VolumeSectionKind,
    ) -> Vec<DeviceInfo> {
        match kind {
            VolumeSectionKind::Output => build_output_device_list(
                &visible_output_devices(&audio.output_devices.get()),
                &audio.default_output.get(),
            ),
            VolumeSectionKind::Input => {
                build_input_device_list(&audio.input_devices.get(), &audio.default_input.get())
            }
        }
    }
}

/// Outputs shown in the picker, in row order.
///
/// Outputs whose ports are all unavailable (e.g. HDMI/DisplayPort with nothing
/// connected) are hidden. Portless outputs, such as virtual sinks, stay visible.
pub(super) fn visible_output_devices(devices: &[Arc<OutputDevice>]) -> Vec<Arc<OutputDevice>> {
    devices
        .iter()
        .filter(|device| has_available_port(&device.ports.get()))
        .cloned()
        .collect()
}

fn has_available_port(ports: &[DevicePort]) -> bool {
    ports.is_empty() || ports.iter().any(|port| port.available)
}

pub(super) fn build_output_device_list(
    devices: &[Arc<OutputDevice>],
    default: &Option<Arc<OutputDevice>>,
) -> Vec<DeviceInfo> {
    devices
        .iter()
        .map(|device| DeviceInfo {
            description: device.description.get(),
            subtitle: helpers::active_port_description(
                &device.active_port.get(),
                &device.ports.get(),
            ),
            icon: helpers::output_device_icon(
                &device.name.get(),
                &device.description.get(),
                &device.properties.get(),
            ),
            is_active: default
                .as_ref()
                .is_some_and(|default_device| default_device.key == device.key),
        })
        .collect()
}

pub(super) fn build_input_device_list(
    devices: &[Arc<InputDevice>],
    default: &Option<Arc<InputDevice>>,
) -> Vec<DeviceInfo> {
    devices
        .iter()
        .filter(|device| !device.is_monitor.get())
        .map(|device| DeviceInfo {
            description: device.description.get(),
            subtitle: helpers::active_port_description(
                &device.active_port.get(),
                &device.ports.get(),
            ),
            icon: helpers::input_device_icon(
                &device.name.get(),
                &device.description.get(),
                &device.properties.get(),
            ),
            is_active: default
                .as_ref()
                .is_some_and(|default_device| default_device.key == device.key),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn port(available: bool) -> DevicePort {
        DevicePort {
            name: String::new(),
            description: String::new(),
            priority: 0,
            available,
        }
    }

    #[test]
    fn portless_outputs_are_available() {
        assert!(has_available_port(&[]));
    }

    #[test]
    fn outputs_with_only_unavailable_ports_are_hidden() {
        assert!(!has_available_port(&[port(false), port(false)]));
    }

    #[test]
    fn outputs_with_any_available_port_are_shown() {
        assert!(has_available_port(&[port(false), port(true)]));
    }
}
