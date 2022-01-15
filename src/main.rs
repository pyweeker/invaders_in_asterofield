#![allow(clippy::too_many_arguments)]
mod arena;
mod background;
mod components;
mod contact;
mod explosion;
mod laser;
mod player;
mod state;
mod ui;
//++++++++++++++
mod enemy;


//_____________

mod prelude {
    pub use crate::arena::*;
    pub use crate::background::*;
    pub use crate::components::*;
    pub use crate::contact::*;
    pub use crate::explosion::*;
    pub use crate::laser::*;
    pub use crate::player::*;
    pub use crate::state::*;
    pub use crate::ui::*;
    //+++++++++++++++++++++++++++++++++
    pub use crate::enemy::*;
    //_________________________________
    pub use bevy::prelude::*;
    pub use heron::prelude::*;
    pub use rand::{thread_rng, Rng};
}

use crate::prelude::*;

//++++++++++++++++++++++++ CST   

const PLAYER_SPRITE_META: (&str, (f32, f32)) = ("player_a_01.png", (144.0, 75.0));
const PLAYER_LASER_SPRITE_META: (&str, (f32, f32)) = ("laser_a_01.png", (9.0, 54.0));
const ENEMY_SPRITE_META: (&str, (f32, f32)) = ("enemy_a_01.png", (93.0, 84.0));
const ENEMY_LASER_SPRITE_META: (&str, (f32, f32)) = ("laser_b_01.png", (17.0, 55.0));
const EXPLOSION_SHEET: &str = "explo_a_sheet.png";
const SCALE: f32 = 0.5;
const TIME_STEP: f32 = 1. / 60.;
const MAX_ENEMIES: u32 = 4;
const MAX_FORMATION_MEMBERS: u32 = 2;
const PLAYER_RESPAWN_DELAY: f64 = 2.;
//+++++++++++++++++++++ no components

// region:    Resources
pub struct Art {
    player: Handle<Image>,
    player_laser: Handle<Image>,
    enemy: Handle<Image>,
    enemy_laser: Handle<Image>,
    explosion: Handle<TextureAtlas>,
}
struct WinSize {
    #[allow(unused)]
    w: f32,
    h: f32,
}
struct ActiveEnemies(u32);

struct PlayerState {
    on: bool,
    last_shot: f64,
}
impl Default for PlayerState {
    fn default() -> Self {
        Self {
            on: false,
            last_shot: 0.,
        }
    }
}
impl PlayerState {
    fn shot(&mut self, time: f64) {
        self.on = false;
        self.last_shot = time;
    }
    fn spawned(&mut self) {
        self.on = true;
        self.last_shot = 0.
    }
}
// endregion: Resources



// +++++++++++++++++++// region:    Components
#[derive(Component)]
struct Laser;

#[derive(Component)]
struct Player;
#[derive(Component)]
struct PlayerReadyFire(bool);
#[derive(Component)]
struct FromPlayer;

#[derive(Component)]
struct Enemy;
#[derive(Component)]
struct FromEnemy;

#[derive(Component)]
struct Explosion;
#[derive(Component)]
struct ExplosionToSpawn(Vec3);


#[derive(Component)]
struct Speed(f32);
impl Default for Speed {
    fn default() -> Self {
        Self(500.)
    }
}


// ____________________
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Kataster".to_string(),
            width: WINDOW_WIDTH as f32,
            height: WINDOW_HEIGHT as f32,
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb_u8(0, 0, 0)))

        .insert_resource(ActiveEnemies(0))   //+++++++++++++++++++++++


        .add_event::<AsteroidSpawnEvent>()
        .add_event::<ExplosionSpawnEvent>()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(BackgroundPlugin {})
        //+++++++++++++++++++++++++++++++++++++++++++
        .add_plugin(EnemyPlugin)
        //____________________________________
        .add_state(AppState::StartMenu)
        .add_system_set(
            SystemSet::on_enter(AppState::StartMenu)
                .with_system(start_menu.system())
                .with_system(appstate_enter_despawn.system()),
        )
        .add_system_set(
            SystemSet::on_enter(AppState::Game)
                .with_system(setup_arena.system())
                .with_system(game_ui_spawn.system())
                .with_system(appstate_enter_despawn.system()),
        )
        .add_system_set(
            SystemSet::on_update(AppState::Game)
                .with_system(position_system.system())
                .with_system(player_dampening_system.system())
                .with_system(ship_cannon_system.system())
                .with_system(despawn_laser_system.system())
                .with_system(contact_system.system())
                .with_system(arena_asteroids.system())
                .with_system(spawn_asteroid_event.system())
                .with_system(score_ui_system.system())
                .with_system(life_ui_system.system()),
        )
        .add_state(AppGameState::Invalid)
        .add_system_set(
            SystemSet::on_enter(AppGameState::Pause)
                .with_system(pause_menu.system())
                .with_system(appgamestate_enter_despawn.system()),
        )
        .add_system_set(
            SystemSet::on_enter(AppGameState::GameOver)
                .with_system(gameover_menu.system())
                .with_system(appgamestate_enter_despawn.system()),
        )
        .add_system_set(
            SystemSet::on_enter(AppGameState::Invalid)
                .with_system(appgamestate_enter_despawn.system()),
        )
        .add_system_set(
            SystemSet::on_enter(AppGameState::Game)
                .with_system(appgamestate_enter_despawn.system()),
        )
        .add_system(user_input_system.system())
        .add_system(handle_explosion.system())
        .add_system(draw_blink_system.system())
        .add_system(spawn_explosion_event.system())
        .add_startup_system(setup.system())
        .run();
}

/// UiCamera and Camera2d are spawn once and for all.
/// Despawning them does not seem to be the way to go in bevy.
pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>,

    //+++++++++++++++++
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut windows: ResMut<Windows>,

    ) {

    //++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++

    let window = windows.get_primary_mut().unwrap();

    //++________________________________
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform = Transform {
        scale: Vec3::splat(CAMERA_SCALE),
        ..Default::default()
    };
    commands.spawn_bundle(camera);
    commands.spawn_bundle(UiCameraBundle::default());
    commands.insert_resource(RunState::new(&asset_server));

    //++++++++++++++++++++++++
    let (player_sprite_name, _) = PLAYER_SPRITE_META;
    let (player_laser_sprite_name, _) = PLAYER_LASER_SPRITE_META;
    let (enemy_sprite_name, _) = ENEMY_SPRITE_META;
    let (enemy_laser_sprite_name, _) = ENEMY_LASER_SPRITE_META;

    let texture_handle = asset_server.load(EXPLOSION_SHEET);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(64.0, 64.0), 4, 4);



    commands.insert_resource(Art {
        player: asset_server.load(player_sprite_name),
        player_laser: asset_server.load(player_laser_sprite_name),
        enemy: asset_server.load(enemy_sprite_name),
        enemy_laser: asset_server.load(enemy_laser_sprite_name),
        explosion: texture_atlases.add(texture_atlas),
    });

    commands.insert_resource(WinSize {
        w: window.width(),
        h: window.height(),
    });

    //_________________
}
