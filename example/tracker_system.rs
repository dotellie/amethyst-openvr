use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::components::{GlobalTransform, Transform};
use amethyst::core::shrev::EventChannel;
use amethyst::core::specs::prelude::*;
use amethyst::renderer::{Material, MaterialDefaults, Mesh, PosNormTangTex, Shape, Texture};
use amethyst::xr::components::TrackingDevice;
use amethyst::xr::XREvent;

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
    );

    fn run(&mut self, system_data: Self::SystemData) {
        let (entities, updater, xr_events, loader, meshes, textures, material_defaults, trackers) =
            system_data;

        for event in xr_events.read(self.xr_event_reader.as_mut().unwrap()) {
            match event {
                XREvent::TrackerAdded(tracker) => {
                    let entity = entities.create();

                    let mut tracker = tracker.clone();

                    updater.insert(entity, GlobalTransform::default());
                    updater.insert(entity, Transform::default());

                    if !tracker.has_model() {
                        // Add default mesh and material if tracker doesn't have any
                        let mesh_data = Shape::Cylinder(32, None)
                            .generate::<Vec<PosNormTangTex>>(Some((0.1, 0.1, 0.1)));
                        let mesh = loader.load_from_data(mesh_data, (), &meshes);

                        let albedo = loader.load_from_data([1.0; 4].into(), (), &textures);

                        let material = Material {
                            albedo,
                            ..material_defaults.0.clone()
                        };

                        updater.insert(entity, mesh);
                        updater.insert(entity, material);
                    } else {
                        tracker.set_render_model_enabled(true);
                    }

                    updater.insert(entity, tracker);
                }
                XREvent::TrackerModelLoaded(index) => {
                    for (entity, tracker) in (&*entities, &trackers).join() {
                        if tracker.id() == *index {
                            let mesh = tracker.mesh().unwrap();

                            let texture = tracker.texture().unwrap_or_else(|| loader.load_from_data([1.0; 4].into(), (), &textures));

                            let material = Material {
                                albedo: texture,
                                ..material_defaults.0.clone()
                            };

                            updater.insert(entity, mesh);
                            updater.insert(entity, material);
                        }
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
