#[macro_use]
extern crate log;
extern crate amethyst;
extern crate openvr;
extern crate openvr_sys;

pub use openvr::ApplicationType;

use amethyst::{Result, Error};
use amethyst::core::cgmath::{Vector3, Quaternion};

use amethyst::xr::{XRBackend, TrackerPositionData};
use openvr::{
    Context, System, Compositor, RenderModels, TrackedDevicePoses, TrackedDevicePose,
    TrackingUniverseOrigin, init,
};

pub struct OpenVR {
    context: Context,
    system: System,
    compositor: Compositor,
    render_models: RenderModels,

    tracked_device_poses: Option<TrackedDevicePoses>,

    registered_trackers: Option<[bool; 16]>,
}

impl OpenVR {
    pub fn is_available() -> bool {
        unsafe {
            openvr_sys::VR_IsHmdPresent()
        }
    }

    pub fn init(application_type: ApplicationType) -> Result<OpenVR> {
        // TODO: Handle unsafe
        let context = unsafe { init(application_type).map_err(|_| Error::Application)? };
        let system = context.system().map_err(|_| Error::Application)?;
        let compositor = context.compositor().map_err(|_| Error::Application)?;
        let render_models = context.render_models().map_err(|_| Error::Application)?;

        Ok(OpenVR {
            context,
            system,
            compositor,
            render_models,

            tracked_device_poses: None,

            registered_trackers: None,
        })
    }
}

impl XRBackend for OpenVR {
    fn wait(&mut self) {
        use TrackingUniverseOrigin::Standing;
        while let Some((event_info, _)) = self.system.poll_next_event_with_pose(Standing) {
            println!("{:?}", event_info.event);
            match event_info.event {
                _ => (),
            }
        }

        if let Ok(poses) = self.compositor.wait_get_poses() {
            self.tracked_device_poses = Some(poses.render);
        } else {
            warn!("OpenVR compositor failed to wait");
        }
    }

    fn get_new_trackers(&mut self) -> Option<Vec<u32>> {
        if let Some(ref mut registered_trackers) = self.registered_trackers {
            let mut tracker_data = None;

            if let Some(poses) = self.tracked_device_poses {
                for i in 0..16 {
                    if !registered_trackers[i] && poses[i].device_is_connected() {
                        if tracker_data.is_none() {
                            tracker_data = Some(Vec::new());
                        }

                        registered_trackers[i] = true;
                        tracker_data.as_mut().unwrap().push(i as u32);
                    }
                }
            }

            tracker_data
        } else {
            let mut trackers = [false; 16];
            let mut tracker_data = Vec::new();

            if let Some(poses) = self.tracked_device_poses {
                for i in 0..16 {
                    let pose = poses[i];

                    let connected = pose.device_is_connected();
                    trackers[i] = connected;
                    if connected {
                        tracker_data.push(i as u32);
                    }
                }
            }

            self.registered_trackers = Some(trackers);
            Some(tracker_data)
        }
    }

    fn get_removed_trackers(&mut self) -> Option<Vec<u32>> {
        if let Some(ref mut registered_trackers) = self.registered_trackers {
            let mut removed_trackers = None;

            if let Some(poses) = self.tracked_device_poses {
                for i in 0..16 {
                    if registered_trackers[i] && !poses[i].device_is_connected() {
                        if removed_trackers.is_none() {
                            removed_trackers = Some(Vec::new());
                        }

                        registered_trackers[i] = false;
                        removed_trackers.as_mut().unwrap().push(i as u32);
                    }
                }
            }

            return removed_trackers;
        }
        None
    }

    fn get_tracker_position(&mut self, index: u32) -> TrackerPositionData {
        if let Some(poses) = self.tracked_device_poses {
            let pose = poses[index as usize];

            let (p, q) = {
                let mut m = pose.device_to_absolute_tracking();

                let p = [m[0][3], m[1][3], m[2][3]];

                let mut q = [
                    (f32::max(0.0, 1.0 + m[0][0] + m[1][1] + m[2][2])).sqrt() / 2.0,
                    (f32::max(0.0, 1.0 + m[0][0] - m[1][1] - m[2][2])).sqrt() / 2.0,
                    (f32::max(0.0, 1.0 - m[0][0] + m[1][1] - m[2][2])).sqrt() / 2.0,
                    (f32::max(0.0, 1.0 - m[0][0] - m[1][1] + m[2][2])).sqrt() / 2.0,
                ];
                q[1] = copysign(q[1], m[2][1] - m[1][2]);
                q[2] = copysign(q[2], m[0][2] - m[2][0]);
                q[3] = copysign(q[3], m[1][0] - m[0][1]);

                (p, q)
            };
            let v = pose.velocity();
            let av = pose.angular_velocity();

            let position = Vector3::new(p[0], p[1], p[2]);
            let rotation = Quaternion::new(q[0], q[1], q[2], q[3]);
            let velocity = Vector3::new(v[0], v[1], v[2]);
            let angular_velocity = Vector3::new(av[0], av[1], av[2]);

            TrackerPositionData {
                position,
                rotation,
                velocity,
                angular_velocity,
                valid: pose.device_is_connected() && pose.pose_is_valid(),
            }
        } else {
            let vec_zero = Vector3::new(0.0, 0.0, 0.0);
            let rot_zero = Quaternion::new(0.0, 0.0, 0.0, 0.0);

            TrackerPositionData {
                position: vec_zero,
                rotation: rot_zero,
                velocity: vec_zero,
                angular_velocity: vec_zero,
                valid: false,
            }
        }
    }

    fn get_area(&mut self) -> Vec<[f32; 3]> {
        unimplemented!()
    }

    fn get_hidden_area_mesh(&mut self) -> Vec<[f32; 3]> {
        unimplemented!()
    }
}

#[inline]
pub fn copysign(a: f32, b: f32) -> f32 {
    if b == 0.0 {
        0.0
    } else {
        a.abs() * b.signum()
    }
}
