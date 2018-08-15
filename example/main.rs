extern crate amethyst;
extern crate amethyst_openvr;

mod tracker_system;

use amethyst::core::cgmath::{Deg, Matrix4};
use amethyst::core::transform::{GlobalTransform, Transform, TransformBundle};
use amethyst::input::{is_close_requested, is_key_down, InputBundle};
use amethyst::prelude::*;
use amethyst::renderer::{
    Camera, DrawPbm, Event, Light, PointLight, PosNormTangTex, Projection, VirtualKeyCode,
};
use amethyst::ui::UiBundle;
use amethyst::utils::fps_counter::FPSCounterBundle;
use amethyst::Error;

use amethyst::xr::{XRBundle, XREvent};
use amethyst_openvr::{ApplicationType, OpenVR};

#[derive(Default)]
struct VRExample;

impl<'a, 'b> State<GameData<'a, 'b>> for VRExample {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;

        let transform =
            Matrix4::from_translation([0.0, 7.0, 0.0].into()) * Matrix4::from_angle_x(Deg(-90.));

        world
            .create_entity()
            .with(GlobalTransform(transform.into()))
            .with(Camera::from(Projection::perspective(1.3, Deg(60.0))))
            .build();

        let light1: Light = PointLight {
            intensity: 6.0,
            color: [0.9, 0.9, 0.9].into(),
            ..PointLight::default()
        }.into();

        let light1_transform =
            GlobalTransform(Matrix4::from_translation([0.0, 10.0, 0.0].into()).into());

        world
            .create_entity()
            .with(light1)
            .with(light1_transform)
            .build();
    }

    fn handle_event(&mut self, _: StateData<GameData>, event: Event) -> Trans<GameData<'a, 'b>> {
        if is_close_requested(&event) || is_key_down(&event, VirtualKeyCode::Escape) {
            return Trans::Quit;
        }

        Trans::None
    }

    fn update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>> {
        data.data.update(&data.world);

        Trans::None
    }
}

fn main() -> Result<(), Error> {
    amethyst::start_logger(Default::default());

    let resources_directory = format!("{}/example/resources/", env!("CARGO_MANIFEST_DIR"));

    let display_config_path = format!(
        "{}/example/resources/display_config.ron",
        env!("CARGO_MANIFEST_DIR")
    );

    let mut game_data = GameDataBuilder::default();

    if OpenVR::is_available() {
        let openvr = OpenVR::init(ApplicationType::Scene)?;
        game_data = game_data.with_bundle(XRBundle::new(openvr))?;
    }

    game_data = game_data
        .with(
            tracker_system::TrackerSystem::default(),
            "tracker_system",
            &[],
        )
        .with_bundle(TransformBundle::new())?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(FPSCounterBundle::default())?
        .with_basic_renderer(display_config_path, DrawPbm::<PosNormTangTex>::new(), true)?
        .with_bundle(InputBundle::<String, String>::new())?;

    let mut game = Application::build(resources_directory, VRExample::default())?.build(game_data)?;
    game.run();

    Ok(())
}
