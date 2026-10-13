use std::sync::Arc;

use futures::stream::{StreamExt, select_all};
use relm4::ComponentSender;
use wayle_audio::AudioService;
use wayle_widgets::watch;

use super::{
    DevicePicker,
    messages::DevicePickerCmd,
    methods::{build_input_device_list, build_output_device_list, visible_output_devices},
};
use crate::shell::bar::dropdowns::audio::VolumeSectionKind;

pub(super) fn spawn(
    sender: &ComponentSender<DevicePicker>,
    audio: &Arc<AudioService>,
    kind: VolumeSectionKind,
) {
    match kind {
        VolumeSectionKind::Output => {
            let output_devices = audio.output_devices.clone();
            let default_output = audio.default_output.clone();
            sender.command(move |out, shutdown| async move {
                let mut devices_changed = output_devices.watch().boxed();
                let mut default_changed = default_output.watch().boxed();
                let send_list = || {
                    let list = build_output_device_list(
                        &visible_output_devices(&output_devices.get()),
                        &default_output.get(),
                    );
                    let _ = out.send(DevicePickerCmd::DevicesChanged(list));
                };

                tokio::select! {
                    () = shutdown.wait() => {}
                    () = async {
                        loop {
                            // A port's availability can change without the sink list
                            // changing, so watch every sink's ports and resubscribe
                            // whenever the set of sinks changes.
                            let mut ports_changed = select_all(
                                output_devices
                                    .get()
                                    .iter()
                                    .map(|device| device.ports.watch().boxed())
                                    .collect::<Vec<_>>(),
                            );
                            send_list();

                            loop {
                                tokio::select! {
                                    Some(_) = devices_changed.next() => break,
                                    Some(_) = default_changed.next() => {}
                                    Some(_) = ports_changed.next() => {}
                                    else => return,
                                }
                                send_list();
                            }
                        }
                    } => {}
                }
            });
        }
        VolumeSectionKind::Input => {
            let input_devices = audio.input_devices.clone();
            let default_input = audio.default_input.clone();
            watch!(
                sender,
                [input_devices.watch(), default_input.watch()],
                |out| {
                    let list = build_input_device_list(&input_devices.get(), &default_input.get());
                    let _ = out.send(DevicePickerCmd::DevicesChanged(list));
                }
            );
        }
    }
}
