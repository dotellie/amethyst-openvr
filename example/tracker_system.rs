use amethyst::assets::{AssetStorage, Loader};
use amethyst::core::components::{Transform, GlobalTransform};
use amethyst::core::shrev::EventChannel;
use amethyst::core::specs::prelude::*;
use amethyst::renderer::{Material, MaterialDefaults, Mesh, PosNormTangTex, Shape, Texture};
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
    );

    fn run(&mut self, (entities, updater, xr_events, loader, meshes, textures, material_defaults): Self::SystemData) {
        for event in xr_events.read(self.xr_event_reader.as_mut().unwrap()) {
            match event {
                XREvent::TrackerAdded(tracker) => {
                    let mesh_data = Shape::Cylinder(32, None).generate::<Vec<PosNormTangTex>>(Some((0.1, 0.1, 0.1)));
                    let mesh = loader.load_from_data(mesh_data, (), &meshes);

                    let albedo = loader.load_from_data([1.0; 4].into(), (), &textures);

                    let material = Material {
                        albedo: albedo.clone(),
                        ..material_defaults.0.clone()
                    };

                    let entity = entities.create();
                    updater.insert(entity, tracker.clone());
                    updater.insert(entity, GlobalTransform::default());
                    updater.insert(entity, Transform::default());
                    updater.insert(entity, mesh);
                    updater.insert(entity, material);
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
