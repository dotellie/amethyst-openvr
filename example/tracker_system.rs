use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::components::{Transform};
use amethyst::core::shrev::EventChannel;
use amethyst::core::specs::prelude::*;
use amethyst::renderer::{
    ActiveCamera, Material, MaterialDefaults, Mesh, PosNormTangTex, Shape, Texture,
};
use amethyst::xr::components::TrackingDevice;
use amethyst::xr::XREvent;

use amethyst_xr_models::XRModelEnabled;

#[derive(Default)]
pub struct TrackerSystem {
    xr_event_reader: Option<ReaderId<XREvent>>,
}

impl<'a> System<'a> for TrackerSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, LazyUpdate>,
        Read<'a, EventChannel<::XREvent>>,
        ReadExpect<'a, Loader>,
        Read<'a, AssetStorage<Mesh>>,
        Read<'a, AssetStorage<Texture>>,
        ReadExpect<'a, MaterialDefaults>,
        ReadStorage<'a, TrackingDevice>,
        ReadExpect<'a, ActiveCamera>,
    );

    fn run(&mut self, system_data: Self::SystemData) {
        let (
            entities,
            updater,
            xr_events,
            loader,
            meshes,
            textures,
            material_defaults,
            trackers,
            active_camera,
        ) = system_data;

        for event in xr_events.read(self.xr_event_reader.as_mut().unwrap()) {
            match event {
                XREvent::TrackerAdded(tracker) => {
                    if tracker.capabilities().is_camera {
                        updater.insert(active_camera.entity, tracker.clone());
                    } else {
                        let mut entity =
                            updater.create_entity(&*entities).with(Transform::default());

                        let mut tracker = tracker.clone();

                        if tracker.capabilities().render_model_components == 0 {
                            // Add default mesh and material if tracker doesn't have any
                            let mesh_data = Shape::Cylinder(32, None)
                                .generate::<Vec<PosNormTangTex>>(Some((0.1, 0.1, 0.1)));
                            let mesh = loader.load_from_data(mesh_data, (), &meshes);

                            let albedo = loader.load_from_data([1.0; 4].into(), (), &textures);

                            let material = Material {
                                albedo,
                                ..material_defaults.0.clone()
                            };

                            entity = entity.with(mesh).with(material);
                        } else {
                            entity = entity.with(XRModelEnabled);
                        }

                        entity.with(tracker).build();
                    }
                }
                _ => (),
            }
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);

        self.xr_event_reader = Some(res.fetch_mut::<EventChannel<XREvent>>().register_reader());
    }
}
