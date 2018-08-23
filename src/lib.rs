#[macro_use]
extern crate log;
extern crate amethyst;
extern crate openvr;
extern crate openvr_sys;

pub use openvr::ApplicationType;

use std::ffi::CStr;

use amethyst::core::cgmath::{Quaternion, Vector3};
use amethyst::renderer::{PosNormTangTex, TextureData, TextureMetadata};
use amethyst::{Error, Result};

use amethyst::xr::{
    TrackerCapabilities, TrackerComponentModelInfo, TrackerModelLoadStatus, TrackerPositionData,
    XRBackend,
};
use openvr::{
    init, Compositor, Context, RenderModels, System, TrackedDeviceClass, TrackedDevicePose,
    TrackedDevicePoses, TrackingUniverseOrigin,
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
        unsafe { openvr_sys::VR_IsHmdPresent() }
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

    fn load_model(&self, model_name: &CStr) -> Result<Option<TrackerComponentModelInfo>> {
        if let Some(model) = self.render_models.load_render_model(&render_model_name)? {
            if let Some(texture_id) = model.diffuse_texture_id() {
                if let Ok(maybe_texture) = self.render_models.load_texture(texture_id) {
                    if let Some(texture) = maybe_texture {
                        let vertices = convert_vertices(model.vertices());
                        let indices = model.indices().to_vec();

                        let (w, h) = texture.dimensions();
                        // TODO: specify format
                        let texture = TextureData::U8(
                            texture.data().to_vec(),
                            TextureMetadata::default().with_size(w, h),
                        );

                        Ok(Some(TrackerComponentModelInfo {
                            component_name: model_name.into_string().ok(),
                            vertices,
                            indices,
                            texture: Some(texture),
                        }))
                    } else {
                        Ok(None)
                    }
                } else {
                    let vertices = convert_vertices(model.vertices());
                    let indices = model.indices().to_vec();

                    Ok(Some(TrackerComponentModelInfo {
                        component_name: model_name.into_string().ok(),
                        vertices,
                        indices,
                        texture: None,
                    }))
                }
            } else {
                let vertices = convert_vertices(model.vertices());
                let indices = model.indices().to_vec();

                Ok(Some(TrackerComponentModelInfo {
                    component_name: model_name.into_string().ok(),
                    vertices,
                    indices,
                    texture: None,
                }))
            }
        } else {
            Ok(None)
        }
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

    fn get_tracker_model(&mut self, index: u32) -> TrackerModelLoadStatus {
        let render_model_name = if let Ok(name) = self.system.string_tracked_device_property(
            index,
            openvr_sys::ETrackedDeviceProperty_Prop_RenderModelName_String,
        ) {
            name
        } else {
            return TrackerModelLoadStatus::Unavailable;
        };

        let component_count = self.render_models.component_count(&render_model_name);

        if component_count == 0 {
            if let Ok(maybe_model_info) = self.load_model(render_model_name) {
                if let Some(mut model_info) = maybe_model_info {
                    // A complete render model isn't a component
                    model_info.component_name = None;
                    TrackerModelLoadStatus::Available(vec![model_info])
                } else {
                    TrackerModelLoadStatus::Pending
                }
            } else {
                TrackerModelLoadStatus::Unavailable
            }
        } else {
            let load_result: Result<_> = (0..component_count)
                .map(|n| {
                    self.render_models
                        .component_name(&render_model_name, n)
                        .unwrap()
                })
                .map(|component_name| self.load_model(component_name))
                .collect();

            if let Ok(maybe_models) = load_result {
                let i: i32 = maybe_models;
            }
        }
    }

    fn get_tracker_capabilities(&mut self, index: u32) -> TrackerCapabilities {
        let has_render_model = self
            .system
            .string_tracked_device_property(
                index,
                openvr_sys::ETrackedDeviceProperty_Prop_RenderModelName_String,
            )
            .is_ok();
        let is_camera = self.system.tracked_device_class(index) == TrackedDeviceClass::HMD;

        TrackerCapabilities {
            has_render_model,
            is_camera,
        }
    }
}

#[inline]
fn copysign(a: f32, b: f32) -> f32 {
    if b == 0.0 {
        0.0
    } else {
        a.abs() * b.signum()
    }
}

#[inline]
fn convert_vertices(vertices: &[openvr::render_models::Vertex]) -> Vec<PosNormTangTex> {
    vertices
        .iter()
        .map(|vert| {
            let normal_vector = Vector3::from(vert.normal);
            let up = Vector3::from([0.0, 1.0, 0.0]);
            let tangent = normal_vector.cross(up).cross(normal_vector).into();
            let [u, v] = vert.texture_coord;
            PosNormTangTex {
                position: vert.position,
                normal: vert.normal,
                tangent,
                tex_coord: [u, 1.0 - v],
            }
        })
        .collect()
}
