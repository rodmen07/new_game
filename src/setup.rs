use crate::components::Furnishings;
use crate::components::{
    ActionKind, ApartmentUnit, BarSmooth, BodyPart, Building, BuildingKind, Collider,
    DayNightOverlay, HobbyKind, HudBar, HudLabel, InteractHighlight, Interactable, ItemKind,
    LocalPlayer, MainCamera, NotifContainer, Npc, NpcId, NpcLabel, NpcPersonality, ObjectSize,
    OwnedPetVisual, PetKind, Player, PlayerId, PlayerIndicator, SkillCareerBar, SkillCookingBar,
    SkillFitnessBar, SkillPanel, SkillSocialBar, TutorialBodyText, TutorialHintText,
    TutorialOverlay, TypingInstruction, TypingLabel, TypingOverlay, TypingOverlayFade,
    TypingRetries, TypingWordCurrent, TypingWordCurrentBox, TypingWordRemaining, TypingWordRow,
    TypingWordRowScale, TypingWordTyped, Vehicle,
};
use crate::resources::{
    ActionPrompt, BankInput, HousingTier, Inventory, PlayerMovement, PlayerStats, Skills,
    VehicleState, WorkStreak,
};
use bevy::prelude::*;
use bevy::render::{
    render_asset::RenderAssetUsages,
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_ecs_tilemap::prelude::*;

use crate::constants::MAP_SCALE;

/// World-space scale multiplier applied inside all layout helpers.
/// All design coordinates are written in pre-scale units; S is applied internally.
const S: f32 = MAP_SCALE;

// ── Tilemap constants ─────────────────────────────────────────────────────────
/// Side length of each tile in the texture atlas (pixels) and in world units
/// (Bevy world pixels).  Because S=4, one tile covers 16 pre-scale units.
const TILE_PX: u32 = 64;
/// Total number of distinct tile types (= columns in the atlas row).
const TILE_COUNT: u32 = TILE_COLORS.len() as u32;
/// Tilemap dimensions in tiles.
const MAP_COLS: u32 = 90;
const MAP_ROWS: u32 = 62;
/// World-space position of the bottom-left corner of tile (0, 0).
/// Chosen so that the main road (pre-scale y=0) is centred at row 25.
const TILEMAP_ORIGIN_X: f32 = -2848.0;
const TILEMAP_ORIGIN_Y: f32 = -1632.0;

// Tile type indices — correspond to columns in the generated atlas texture.
const T_GROUND: u32 = 0;
const T_ROAD: u32 = 1;
const T_SIDEWALK: u32 = 2;
const T_ALLEY: u32 = 3;
const T_HOME: u32 = 4;
const T_GYM: u32 = 5; // formerly T_WELLNESS
const T_LIBRARY: u32 = 6;
const T_PARK: u32 = 7;
const T_OFFICE: u32 = 8;
const T_BANK: u32 = 9;
const T_HOSPITAL: u32 = 10; // formerly T_CLINIC
const T_MARKET: u32 = 11; // formerly T_STORE
const T_RESTAURANT: u32 = 12; // formerly T_CAFE
const T_ADOPTION: u32 = 13;
const T_GARAGE: u32 = 14;
const T_APARTMENTS: u32 = 15;
// April 2026 map redesign additions:
const T_SCHOOL: u32 = 16;
const T_TRANSIT: u32 = 17;

/// sRGB u8 colour for each tile type (matches the original Bevy Color::srgb values).
const TILE_COLORS: [[u8; 3]; 18] = [
    [71, 66, 59],    // 0  Ground
    [92, 87, 77],    // 1  Road
    [107, 102, 92],  // 2  Sidewalk
    [87, 82, 71],    // 3  Alley
    [184, 148, 107], // 4  HOME
    [89, 158, 140],  // 5  GYM (was WELLNESS)
    [77, 107, 148],  // 6  LIBRARY
    [71, 148, 71],   // 7  PARK
    [107, 133, 173], // 8  OFFICE
    [140, 122, 82],  // 9  BANK
    [217, 230, 224], // 10 HOSPITAL (was CLINIC)
    [82, 133, 148],  // 11 MARKET (was STORE)
    [209, 173, 115], // 12 RESTAURANT (was CAFÉ)
    [158, 128, 199], // 13 ADOPTION
    [102, 97, 115],  // 14 GARAGE
    [158, 140, 199], // 15 APARTMENTS
    [199, 87, 71],   // 16 SCHOOL
    [128, 140, 153], // 17 TRANSIT
];

/// Convert a pre-scale x coordinate to the nearest tilemap column index.
/// Coordinates west of TILEMAP_ORIGIN_X are clamped to column 0.
fn pre_to_col(pre_x: f32) -> u32 {
    (((pre_x * S) - TILEMAP_ORIGIN_X) / TILE_PX as f32).max(0.0) as u32
}

/// Convert a pre-scale y coordinate to the nearest tilemap row index.
/// Coordinates south of TILEMAP_ORIGIN_Y are clamped to row 0.
fn pre_to_row(pre_y: f32) -> u32 {
    (((pre_y * S) - TILEMAP_ORIGIN_Y) / TILE_PX as f32).max(0.0) as u32
}

/// Fill a rectangular region of the flat tile grid (row-major, row 0 = bottom).
fn fill_tiles(grid: &mut [u32], x_min: f32, x_max: f32, y_min: f32, y_max: f32, tile: u32) {
    let c0 = pre_to_col(x_min).min(MAP_COLS.saturating_sub(1));
    let c1 = (pre_to_col(x_max) + 1).min(MAP_COLS);
    let r0 = pre_to_row(y_min).min(MAP_ROWS.saturating_sub(1));
    let r1 = (pre_to_row(y_max) + 1).min(MAP_ROWS);
    for r in r0..r1 {
        for c in c0..c1 {
            grid[(r * MAP_COLS + c) as usize] = tile;
        }
    }
}

/// Build the flat tile grid that describes every terrain and zone surface.
/// Fills happen lowest-priority first; later fills override earlier ones.
fn build_tile_grid() -> Vec<u32> {
    let mut g = vec![T_GROUND; (MAP_COLS * MAP_ROWS) as usize];

    // Side alleys (east and west of the main grid)
    fill_tiles(&mut g, -692., -508., -200., 380., T_ALLEY);
    fill_tiles(&mut g, 508., 692., -200., 380., T_ALLEY);

    // Main horizontal road + flanking sidewalks
    fill_tiles(&mut g, -720., 720., 65., 78., T_SIDEWALK);
    fill_tiles(&mut g, -720., 720., -78., -65., T_SIDEWALK);
    fill_tiles(&mut g, -720., 720., -55., 55., T_ROAD);

    // Back road (north of buildings) + flanking sidewalks
    fill_tiles(&mut g, -720., 720., 290., 303., T_SIDEWALK);
    fill_tiles(&mut g, -720., 720., 357., 370., T_SIDEWALK);
    fill_tiles(&mut g, -720., 720., 303., 357., T_ROAD);

    // North-row building zone floors
    fill_tiles(&mut g, -515., -335., 70., 290., T_HOME);
    fill_tiles(&mut g, -330., -180., 80., 280., T_GYM);
    fill_tiles(&mut g, -175., 5., 70., 290., T_LIBRARY);
    fill_tiles(&mut g, 10., 160., 100., 260., T_PARK);
    fill_tiles(&mut g, 335., 515., 70., 290., T_OFFICE);

    // South-row building zone floors
    fill_tiles(&mut g, -500., -350., -280., -80., T_BANK);
    fill_tiles(&mut g, -330., -180., -280., -80., T_HOSPITAL);
    fill_tiles(&mut g, -160., -10., -280., -80., T_MARKET);
    fill_tiles(&mut g, 10., 160., -280., -80., T_RESTAURANT);
    fill_tiles(&mut g, 180., 330., -280., -80., T_ADOPTION);
    fill_tiles(&mut g, 350., 500., -280., -80., T_GARAGE);

    // Apartments complex (north of back road)
    fill_tiles(&mut g, -250., 250., 380., 540., T_APARTMENTS);

    // Back-street additions (April 2026): SCHOOL west of apartments,
    // TRANSIT station east of apartments.
    fill_tiles(&mut g, -525., -375., 380., 540., T_SCHOOL);
    fill_tiles(&mut g, 375., 525., 380., 540., T_TRANSIT);

    g
}

/// Cheap deterministic pseudo-random in 0..=255 from three integer inputs.
/// Used to seed per-tile pixel noise so the texture is reproducible across runs.
fn pixel_hash(tile_idx: u32, px: u32, py: u32) -> i32 {
    let mut x = px
        .wrapping_mul(374_761_393)
        .wrapping_add(py.wrapping_mul(668_265_263))
        .wrapping_add(tile_idx.wrapping_mul(2_147_483_647));
    x = (x ^ (x >> 13)).wrapping_mul(1_274_126_177);
    ((x ^ (x >> 16)) & 0xFF) as i32
}

/// Add or subtract `d` from `c`, clamped to 0..=255.
fn adjust(c: u8, d: i32) -> u8 {
    (c as i32 + d).clamp(0, 255) as u8
}

/// Per-pixel colour for a tile, applying tile-type-specific patterns over
/// the base zone colour. Patterns stay within the 64x64 tile so seams
/// between adjacent tiles remain clean.
fn tile_pixel_color(tile_idx: u32, px: u32, py: u32, base: [u8; 3]) -> [u8; 3] {
    let h = pixel_hash(tile_idx, px, py);
    // Low-amplitude luminance noise, ~ -8..+7.
    let noise = (h - 128) / 16;
    let mut out = [
        adjust(base[0], noise),
        adjust(base[1], noise),
        adjust(base[2], noise),
    ];

    match tile_idx {
        T_ROAD => {
            // Asphalt speckle: occasional darker grit.
            if h < 24 {
                out = [
                    adjust(out[0], -18),
                    adjust(out[1], -18),
                    adjust(out[2], -18),
                ];
            }
        }
        T_SIDEWALK => {
            // Concrete panel seams every 16 px along x, plus a faint horizontal mid-line.
            if px.is_multiple_of(16) {
                out = [
                    adjust(out[0], -22),
                    adjust(out[1], -22),
                    adjust(out[2], -22),
                ];
            }
            if py == 0 || py == 32 || py == TILE_PX - 1 {
                out = [
                    adjust(out[0], -12),
                    adjust(out[1], -12),
                    adjust(out[2], -12),
                ];
            }
        }
        T_ALLEY | T_GROUND => {
            // Apply these relative to `out` so earlier per-pixel noise is preserved.
            if h < 28 {
                out = [
                    adjust(out[0], -22),
                    adjust(out[1], -22),
                    adjust(out[2], -22),
                ];
            } else if h > 232 {
                out = [adjust(out[0], 18), adjust(out[1], 14), adjust(out[2], 10)];
            }
        }
        T_PARK => {
            // Grass tufts: bright green speckle on top of the current grass color.
            if h > 210 {
                out = [adjust(out[0], -12), adjust(out[1], 28), adjust(out[2], -12)];
            } else if h < 30 {
                out = [adjust(out[0], -8), adjust(out[1], -16), adjust(out[2], -8)];
            }
        }
        T_HOME | T_OFFICE | T_BANK | T_LIBRARY | T_APARTMENTS | T_SCHOOL => {
            // Faint 32x32 floor-panel grid for indoor zone floors.
            if px.is_multiple_of(32) || py.is_multiple_of(32) {
                out = [
                    adjust(out[0], -10),
                    adjust(out[1], -10),
                    adjust(out[2], -10),
                ];
            }
        }
        T_TRANSIT | T_GARAGE => {
            // Concrete platform with a faint diagonal scuff pattern.
            if (px + py).is_multiple_of(16) {
                out = [
                    adjust(out[0], -14),
                    adjust(out[1], -14),
                    adjust(out[2], -14),
                ];
            }
        }
        _ => {
            // Generic shop/zone floor: very subtle 16x16 tile lines.
            if px.is_multiple_of(16) || py.is_multiple_of(16) {
                out = [adjust(out[0], -6), adjust(out[1], -6), adjust(out[2], -6)];
            }
        }
    }
    out
}

/// Create the tileset atlas programmatically: an RGBA image whose
/// `TILE_COUNT` columns are 64x64-pixel textured tiles, one per tile type.
fn create_tileset_image(images: &mut Assets<Image>) -> Handle<Image> {
    let width = (TILE_PX * TILE_COUNT) as usize;
    let height = TILE_PX as usize;
    let mut data = vec![0u8; width * height * 4];
    for (i, base) in TILE_COLORS.iter().enumerate() {
        let x0 = i * TILE_PX as usize;
        for py in 0..height {
            for px in 0..TILE_PX as usize {
                let [r, g, b] = tile_pixel_color(i as u32, px as u32, py as u32, *base);
                let off = (py * width + x0 + px) * 4;
                data[off] = r;
                data[off + 1] = g;
                data[off + 2] = b;
                data[off + 3] = 255;
            }
        }
    }
    images.add(Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    ))
}

/// Spawn the ECS tilemap entity that renders all terrain, roads, and building floors.
/// This replaces the old approach of spawning hundreds of individual `rect()` sprites.
fn spawn_tilemap(commands: &mut Commands, images: &mut Assets<Image>) {
    let tileset = create_tileset_image(images);
    let grid = build_tile_grid();

    let map_size = TilemapSize {
        x: MAP_COLS,
        y: MAP_ROWS,
    };
    let tile_size = TilemapTileSize {
        x: TILE_PX as f32,
        y: TILE_PX as f32,
    };
    let grid_size = TilemapGridSize {
        x: TILE_PX as f32,
        y: TILE_PX as f32,
    };
    let map_type = TilemapType::Square;

    let mut tile_storage = TileStorage::empty(map_size);
    let tilemap_entity = commands.spawn_empty().id();

    for row in 0..MAP_ROWS {
        for col in 0..MAP_COLS {
            let tile_type = grid[(row * MAP_COLS + col) as usize];
            let tile_pos = TilePos { x: col, y: row };
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(tile_type),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(tileset),
        tile_size,
        transform: Transform::from_xyz(TILEMAP_ORIGIN_X, TILEMAP_ORIGIN_Y, 0.0),
        ..Default::default()
    });
}

/// Spawn visual details that sit on top of the tilemap:
/// road markings (edge lines, centre dashes) and lamp posts.
fn spawn_road_details(commands: &mut Commands) {
    // Main road edge lines
    rect(
        commands,
        0.,
        55.,
        3000.,
        2.,
        Color::srgba(1., 1., 0.8, 0.10),
        0.6,
    );
    rect(
        commands,
        0.,
        -55.,
        3000.,
        2.,
        Color::srgba(1., 1., 0.8, 0.10),
        0.6,
    );
    // Main road centre dashes
    for i in -17i32..=17 {
        let x = i as f32 * 40.;
        rect(
            commands,
            x,
            0.,
            18.,
            3.,
            Color::srgba(1., 1., 0.75, 0.20),
            0.7,
        );
    }
    // Lamp posts on main-road sidewalks
    for &(lx, ly) in &[
        (-340., 76.),
        (-170., 76.),
        (0., 76.),
        (170., 76.),
        (340., 76.),
        (-340., -76.),
        (-170., -76.),
        (0., -76.),
        (170., -76.),
        (340., -76.),
    ] {
        lamp_post(commands, lx, ly);
    }
}

/// Builds a composite human figure as child entities of the calling spawn.
/// The root entity should have Transform + Visibility but no Sprite.
///
/// Body layout (local coords, root at 0,0):
///   shadow y=-14*S, feet y=-10*S, legs y=-5*S, torso y=1*S, head y=9*S, hair y=13*S
fn spawn_human(p: &mut ChildBuilder, outfit: Color, pants: Color, skin: Color, hair: Color) {
    // Ground shadow
    p.spawn((
        Sprite {
            color: Color::srgba(0., 0., 0., 0.32),
            custom_size: Some(Vec2::new(20. * S, 7. * S)),
            ..default()
        },
        Transform::from_xyz(2. * S, -14. * S, -1.),
    ));
    // Left shoe
    p.spawn((
        Sprite {
            color: Color::srgb(0.14, 0.10, 0.07),
            custom_size: Some(Vec2::new(4. * S, 4. * S)),
            ..default()
        },
        Transform::from_xyz(-4. * S, -10. * S, 0.5),
        BodyPart::LeftFoot,
    ));
    // Right shoe
    p.spawn((
        Sprite {
            color: Color::srgb(0.14, 0.10, 0.07),
            custom_size: Some(Vec2::new(4. * S, 4. * S)),
            ..default()
        },
        Transform::from_xyz(4. * S, -10. * S, 0.5),
        BodyPart::RightFoot,
    ));
    // Left leg
    p.spawn((
        Sprite {
            color: pants,
            custom_size: Some(Vec2::new(4. * S, 7. * S)),
            ..default()
        },
        Transform::from_xyz(-4. * S, -5. * S, 1.),
        BodyPart::LeftLeg,
    ));
    // Right leg
    p.spawn((
        Sprite {
            color: pants,
            custom_size: Some(Vec2::new(4. * S, 7. * S)),
            ..default()
        },
        Transform::from_xyz(4. * S, -5. * S, 1.),
        BodyPart::RightLeg,
    ));
    // Torso
    p.spawn((
        Sprite {
            color: outfit,
            custom_size: Some(Vec2::new(12. * S, 10. * S)),
            ..default()
        },
        Transform::from_xyz(0., 1. * S, 1.5),
        BodyPart::Body,
    ));
    // Head
    p.spawn((
        Sprite {
            color: skin,
            custom_size: Some(Vec2::new(9. * S, 9. * S)),
            ..default()
        },
        Transform::from_xyz(0., 9. * S, 2.),
        BodyPart::Head,
    ));
    // Hair
    p.spawn((
        Sprite {
            color: hair,
            custom_size: Some(Vec2::new(10. * S, 4. * S)),
            ..default()
        },
        Transform::from_xyz(0., 13. * S, 2.5),
        BodyPart::Hair,
    ));
    // Left eye
    p.spawn((
        Sprite {
            color: Color::srgb(0.08, 0.05, 0.04),
            custom_size: Some(Vec2::new(2. * S, 2. * S)),
            ..default()
        },
        Transform::from_xyz(-2. * S, 9. * S, 3.),
    ));
    // Right eye
    p.spawn((
        Sprite {
            color: Color::srgb(0.08, 0.05, 0.04),
            custom_size: Some(Vec2::new(2. * S, 2. * S)),
            ..default()
        },
        Transform::from_xyz(2. * S, 9. * S, 3.),
    ));
}

pub fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn((Camera2d, MainCamera));
    spawn_tilemap(&mut commands, &mut images);
    spawn_road_details(&mut commands);
    spawn_buildings_and_zones(&mut commands);
    spawn_vehicle(&mut commands);
    spawn_owned_pet(&mut commands);
    spawn_world_objects(&mut commands);
    spawn_npcs(&mut commands);
    spawn_player_entity(&mut commands);
    spawn_collision_walls_and_roads(&mut commands);
    spawn_hud(&mut commands);
    spawn_typing_overlay(&mut commands);
    spawn_skill_panel(&mut commands);
    spawn_tutorial_overlay(&mut commands);
}

fn spawn_buildings_and_zones(commands: &mut Commands) {
    // -- Zone labels (tilemap provides the floor colour) ------------------------
    // North row
    zone_label(commands, -425., 180., 220., "HOME");
    zone_label(commands, -255., 180., 200., "GYM");
    zone_label(commands, -85., 180., 220., "LIBRARY");
    zone_label(commands, 85., 180., 200., "PARK");
    zone_label(commands, 425., 180., 220., "OFFICE");
    // South row
    zone_label(commands, -425., -180., 200., "BANK");
    zone_label(commands, -255., -180., 200., "HOSPITAL");
    zone_label(commands, -85., -180., 200., "MARKET");
    zone_label(commands, 85., -180., 200., "RESTAURANT");
    zone_label(commands, 255., -180., 200., "ADOPTION");
    zone_label(commands, 425., -180., 200., "GARAGE");
    // Back-street row (April 2026 additions)
    zone_label(commands, -450., 460., 160., "SCHOOL");
    zone_label(commands, 450., 460., 160., "TRANSIT");

    // -- Building facade details ------------------------------------------------
    let wc = Color::srgb(0.82, 0.92, 0.98); // window glass

    // HOME (-425, 180, 150x160) - warm residential
    rect(
        commands,
        -425.,
        256.,
        150.,
        10.,
        Color::srgb(0.50, 0.36, 0.22),
        1.15,
    ); // roof ridge
    for wx in [-470., -425., -380.] {
        rect(commands, wx, 225., 22., 16., wc, 1.2);
        rect(
            commands,
            wx,
            225.,
            26.,
            20.,
            Color::srgba(0., 0., 0., 0.18),
            1.18,
        );
    }
    rect(
        commands,
        -425.,
        104.,
        16.,
        28.,
        Color::srgb(0.28, 0.16, 0.06),
        1.2,
    ); // door
    rect(
        commands,
        -425.,
        104.,
        20.,
        32.,
        Color::srgba(0., 0., 0., 0.28),
        1.18,
    );
    for i in 0..3i32 {
        let fy = 120. + i as f32 * 45.;
        rect(
            commands,
            -425.,
            fy,
            140.,
            3.,
            Color::srgb(0.62, 0.46, 0.30),
            1.08,
        );
    }

    // WELLNESS (-255, 180, 150x160) - health/spa
    rect(
        commands,
        -255.,
        256.,
        150.,
        10.,
        Color::srgb(0.25, 0.48, 0.42),
        1.15,
    );
    for wx in [-295., -215.] {
        rect(commands, wx, 225., 22., 16., wc, 1.2);
    }
    rect(
        commands,
        -255.,
        245.,
        14.,
        4.,
        Color::srgb(0.92, 0.18, 0.24),
        1.25,
    ); // cross h
    rect(
        commands,
        -255.,
        242.,
        4.,
        10.,
        Color::srgb(0.92, 0.18, 0.24),
        1.25,
    ); // cross v
    for i in 0..3i32 {
        let wy = 130. + i as f32 * 40.;
        rect(
            commands,
            -255.,
            wy,
            140.,
            3.,
            Color::srgba(0.28, 0.60, 0.54, 0.22),
            1.08,
        );
    }

    // LIBRARY (-85, 180, 150x160) - arched entrance
    rect(
        commands,
        -85.,
        256.,
        150.,
        10.,
        Color::srgb(0.22, 0.30, 0.48),
        1.15,
    );
    rect(
        commands,
        -97.,
        135.,
        10.,
        40.,
        Color::srgb(0.18, 0.25, 0.40),
        1.2,
    ); // pillar L
    rect(
        commands,
        -73.,
        135.,
        10.,
        40.,
        Color::srgb(0.18, 0.25, 0.40),
        1.2,
    ); // pillar R
    rect(
        commands,
        -85.,
        157.,
        36.,
        8.,
        Color::srgb(0.18, 0.25, 0.40),
        1.2,
    ); // arch top
    rect(
        commands,
        -85.,
        103.,
        100.,
        6.,
        Color::srgb(0.26, 0.35, 0.52),
        1.18,
    ); // steps
    for i in 0..3i32 {
        let lx = -145. + i as f32 * 30.;
        rect(
            commands,
            lx,
            170.,
            3.,
            60.,
            Color::srgba(0.20, 0.14, 0.07, 0.35),
            1.08,
        );
    }

    // PARK (85, 180, 150x160) - open green, no facade walls
    rect(
        commands,
        85.,
        110.,
        20.,
        80.,
        Color::srgb(0.44, 0.36, 0.26),
        1.12,
    ); // dirt path
    for (fx, fy, fc) in [
        (30., 250., Color::srgb(0.95, 0.40, 0.55)),
        (50., 245., Color::srgb(1.00, 0.82, 0.20)),
        (140., 250., Color::srgb(0.55, 0.75, 0.95)),
        (120., 245., Color::srgb(0.90, 0.50, 0.80)),
        (30., 140., Color::srgb(1.00, 0.70, 0.25)),
        (140., 140., Color::srgb(0.55, 0.90, 0.55)),
    ] {
        rect(commands, fx, fy, 10., 7., fc, 3.1);
        rect(
            commands,
            fx + 5.,
            fy - 3.,
            7.,
            5.,
            Color::srgb(0.18, 0.52, 0.18),
            3.05,
        );
    }
    rect(
        commands,
        50.,
        165.,
        22.,
        5.,
        Color::srgb(0.50, 0.32, 0.14),
        1.2,
    ); // bench decor
    rect(
        commands,
        50.,
        168.,
        22.,
        3.,
        Color::srgb(0.40, 0.25, 0.10),
        1.21,
    );

    // OFFICE (425, 180, 150x160) - corporate glass
    rect(
        commands,
        425.,
        256.,
        150.,
        10.,
        Color::srgb(0.25, 0.32, 0.45),
        1.15,
    );
    for wx in [380., 410., 440., 470.] {
        for wy in [240., 210., 170.] {
            rect(commands, wx, wy, 16., 12., wc, 1.2);
            rect(
                commands,
                wx,
                wy,
                20.,
                16.,
                Color::srgba(0., 0., 0., 0.15),
                1.18,
            );
        }
    }
    rect(
        commands,
        425.,
        105.,
        30.,
        36.,
        Color::srgb(0.60, 0.82, 0.95),
        1.2,
    ); // glass entrance
    rect(
        commands,
        425.,
        105.,
        34.,
        40.,
        Color::srgba(0., 0., 0., 0.22),
        1.18,
    );
    for i in 0..3i32 {
        let fx = 360. + i as f32 * 45.;
        rect(
            commands,
            fx,
            180.,
            3.,
            140.,
            Color::srgba(0.15, 0.20, 0.30, 0.35),
            1.08,
        );
    }

    // BANK (-425, -180, 150x160) - dignified columns
    rect(
        commands,
        -425.,
        -104.,
        150.,
        10.,
        Color::srgb(0.62, 0.52, 0.30),
        1.15,
    ); // cornice
    for cx in [-475., -450., -400., -375.] {
        rect(
            commands,
            cx,
            -170.,
            8.,
            70.,
            Color::srgb(0.68, 0.60, 0.40),
            1.2,
        );
    }
    for (sy, swidth) in [(-240., 120.), (-246., 130.), (-252., 140.)] {
        rect(
            commands,
            -425.,
            sy,
            swidth,
            6.,
            Color::srgb(0.60, 0.55, 0.42),
            1.18,
        );
    }
    for i in 0..3i32 {
        let fy = -230. + i as f32 * 35.;
        rect(
            commands,
            -425.,
            fy,
            140.,
            3.,
            Color::srgba(0.85, 0.80, 0.68, 0.30),
            1.08,
        );
    }

    // CLINIC (-255, -180, 150x160) - medical
    rect(
        commands,
        -255.,
        -104.,
        150.,
        10.,
        Color::srgb(0.70, 0.75, 0.72),
        1.15,
    );
    rect(commands, -295., -150., 36., 24., wc, 1.2);
    rect(commands, -215., -150., 36., 24., wc, 1.2);
    rect(
        commands,
        -255.,
        -120.,
        14.,
        4.,
        Color::srgb(0.90, 0.18, 0.24),
        1.22,
    ); // cross h
    rect(
        commands,
        -255.,
        -123.,
        4.,
        10.,
        Color::srgb(0.90, 0.18, 0.24),
        1.22,
    ); // cross v
    rect(
        commands,
        -255.,
        -98.,
        36.,
        5.,
        Color::srgb(0.72, 0.78, 0.76),
        1.18,
    ); // ramp

    // STORE (-85, -180, 150x160) - shop facade
    rect(
        commands,
        -85.,
        -104.,
        150.,
        10.,
        Color::srgb(0.22, 0.38, 0.45),
        1.15,
    );
    rect(commands, -125., -140., 42., 30., wc, 1.2); // display L
    rect(commands, -45., -140., 42., 30., wc, 1.2); // display R
    rect(
        commands,
        -85.,
        -110.,
        110.,
        8.,
        Color::srgb(0.85, 0.22, 0.22),
        1.25,
    ); // awning

    // CAFÉ (85, -180, 150x160) - warm eatery
    rect(
        commands,
        85.,
        -104.,
        150.,
        10.,
        Color::srgb(0.60, 0.44, 0.22),
        1.15,
    );
    rect(
        commands,
        85.,
        -114.,
        120.,
        8.,
        Color::srgb(0.88, 0.55, 0.18),
        1.20,
    ); // awning
    rect(commands, 45., -145., 40., 26., wc, 1.2); // window L
    rect(commands, 125., -145., 40., 26., wc, 1.2); // window R
    rect(
        commands,
        85.,
        -98.,
        8.,
        14.,
        Color::srgb(0.40, 0.28, 0.14),
        1.20,
    ); // sign

    // ADOPTION (255, -180, 150x160) - animal shelter
    rect(
        commands,
        255.,
        -104.,
        150.,
        10.,
        Color::srgb(0.52, 0.40, 0.68),
        1.15,
    );
    rect(
        commands,
        210.,
        -120.,
        8.,
        10.,
        Color::srgb(0.48, 0.36, 0.60),
        1.20,
    ); // cat silhouette
    rect(
        commands,
        222.,
        -118.,
        5.,
        7.,
        Color::srgb(0.48, 0.36, 0.60),
        1.20,
    );
    rect(
        commands,
        300.,
        -122.,
        16.,
        9.,
        Color::srgb(0.48, 0.36, 0.60),
        1.20,
    ); // fish
    rect(
        commands,
        220.,
        -225.,
        26.,
        16.,
        Color::srgb(0.55, 0.44, 0.72),
        1.18,
    ); // kennel L
    rect(
        commands,
        220.,
        -216.,
        26.,
        3.,
        Color::srgb(0.40, 0.30, 0.55),
        1.19,
    );
    rect(
        commands,
        290.,
        -225.,
        26.,
        16.,
        Color::srgb(0.55, 0.44, 0.72),
        1.18,
    ); // kennel R
    rect(
        commands,
        290.,
        -216.,
        26.,
        3.,
        Color::srgb(0.40, 0.30, 0.55),
        1.19,
    );

    // GARAGE (425, -180, 150x160) - roller door
    rect(
        commands,
        425.,
        -125.,
        120.,
        70.,
        Color::srgb(0.30, 0.28, 0.34),
        1.2,
    ); // door panel
    for gy in [-110., -125., -140., -155., -170.] {
        rect(
            commands,
            425.,
            gy,
            116.,
            2.,
            Color::srgba(0., 0., 0., 0.30),
            1.3,
        );
    }
    rect(
        commands,
        425.,
        -105.,
        150.,
        40.,
        Color::srgb(0.35, 0.33, 0.38),
        1.05,
    ); // parking area
}

fn spawn_vehicle(commands: &mut Commands) {
    // -- Car entity (near GARAGE, hidden until purchased) -----------------------
    commands
        .spawn((
            Sprite {
                color: Color::srgb(0.72, 0.18, 0.18),
                custom_size: Some(Vec2::new(62. * S, 28. * S)),
                ..default()
            },
            Transform::from_xyz(425. * S, -280. * S, 2.),
            Vehicle,
            Interactable {
                action: ActionKind::EnterVehicle,
                prompt: "[E] Enter car".to_string(),
            },
            ObjectSize(Vec2::new(62. * S, 28. * S)),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Sprite {
                    color: Color::srgba(0., 0., 0., 0.45),
                    custom_size: Some(Vec2::new(66. * S, 32. * S)),
                    ..default()
                },
                Transform::from_xyz(2. * S, -2. * S, -0.05),
            ));
            p.spawn((
                Sprite {
                    color: Color::srgb(0.55, 0.78, 0.94),
                    custom_size: Some(Vec2::new(26., 10.)),
                    ..default()
                },
                Transform::from_xyz(0., 4., 0.1),
            ));
            for (wx, wy) in [(-22., -11.), (22., -11.), (-22., 11.), (22., 11.)] {
                p.spawn((
                    Sprite {
                        color: Color::srgb(0.12, 0.12, 0.14),
                        custom_size: Some(Vec2::new(8., 8.)),
                        ..default()
                    },
                    Transform::from_xyz(wx, wy, 0.05),
                ));
            }
        });
}

fn spawn_owned_pet(commands: &mut Commands) {
    // Pet entity near HOME, hidden until the player adopts one.
    commands
        .spawn((
            Sprite {
                color: Color::srgb(0.70, 0.54, 0.34),
                custom_size: Some(Vec2::new(72., 44.)),
                ..default()
            },
            Transform::from_xyz(-430. * S, 92. * S, 2.0),
            OwnedPetVisual,
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((
                Sprite {
                    color: Color::srgb(0.10, 0.08, 0.08),
                    custom_size: Some(Vec2::new(8., 8.)),
                    ..default()
                },
                Transform::from_xyz(-14., 8., 0.1),
            ));
            p.spawn((
                Sprite {
                    color: Color::srgb(0.10, 0.08, 0.08),
                    custom_size: Some(Vec2::new(8., 8.)),
                    ..default()
                },
                Transform::from_xyz(14., 8., 0.1),
            ));
            p.spawn((
                Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.22),
                    custom_size: Some(Vec2::new(76., 18.)),
                    ..default()
                },
                Transform::from_xyz(0., -20., -0.1),
            ));
        });
}

fn spawn_world_objects(commands: &mut Commands) {
    // -- Park pond --------------------------------------------------------------
    rect(
        commands,
        55.,
        215.,
        40.,
        30.,
        Color::srgb(0.15, 0.38, 0.58),
        1.08,
    );
    rect(
        commands,
        55.,
        215.,
        44.,
        34.,
        Color::srgba(0., 0., 0., 0.28),
        1.06,
    );
    rect(
        commands,
        52.,
        212.,
        22.,
        14.,
        Color::srgba(0.55, 0.80, 0.95, 0.22),
        1.12,
    );

    // -- Zone interior details --------------------------------------------------

    // HOME interior - larger lounge rug, dining corner, and sofa wall
    rect(
        commands,
        -455.,
        232.,
        68.,
        36.,
        Color::srgb(0.55, 0.32, 0.22),
        1.06,
    ); // rug outer
    rect(
        commands,
        -455.,
        232.,
        58.,
        28.,
        Color::srgb(0.62, 0.38, 0.28),
        1.07,
    ); // rug inner
    rect(
        commands,
        -365.,
        156.,
        34.,
        18.,
        Color::srgb(0.42, 0.28, 0.12),
        1.06,
    ); // table
    rect(
        commands,
        -365.,
        156.,
        30.,
        14.,
        Color::srgb(0.50, 0.34, 0.16),
        1.08,
    );
    for cx in [-378., -352.] {
        rect(
            commands,
            cx,
            147.,
            8.,
            8.,
            Color::srgb(0.42, 0.28, 0.12),
            1.06,
        ); // chairs
        rect(
            commands,
            cx,
            165.,
            8.,
            8.,
            Color::srgb(0.42, 0.28, 0.12),
            1.06,
        );
    }
    rect(
        commands,
        -500.,
        186.,
        16.,
        46.,
        Color::srgb(0.48, 0.30, 0.22),
        1.06,
    ); // sofa
    rect(
        commands,
        -500.,
        186.,
        12.,
        40.,
        Color::srgb(0.58, 0.38, 0.28),
        1.08,
    );

    // OFFICE interior - filing cabinet bank on west wall, desk surfaces for back row
    // Filing cabinet bank (west wall)
    rect(
        commands,
        345.,
        200.,
        8.,
        60.,
        Color::srgb(0.32, 0.34, 0.38),
        1.06,
    );
    rect(
        commands,
        345.,
        200.,
        5.,
        56.,
        Color::srgb(0.40, 0.42, 0.46),
        1.07,
    );
    rect(
        commands,
        345.,
        176.,
        5.,
        2.,
        Color::srgb(0.55, 0.56, 0.60),
        1.08,
    );
    rect(
        commands,
        345.,
        200.,
        5.,
        2.,
        Color::srgb(0.55, 0.56, 0.60),
        1.08,
    );
    rect(
        commands,
        345.,
        224.,
        5.,
        2.,
        Color::srgb(0.55, 0.56, 0.60),
        1.08,
    );
    // Back-row desk surfaces (three desks in a line at y=235)
    for dx in [385., 425., 465.] {
        rect(
            commands,
            dx,
            235.,
            30.,
            16.,
            Color::srgb(0.38, 0.30, 0.16),
            1.06,
        );
        rect(
            commands,
            dx,
            235.,
            26.,
            12.,
            Color::srgb(0.52, 0.42, 0.24),
            1.07,
        );
        rect(
            commands,
            dx - 8.,
            237.,
            10.,
            7.,
            Color::srgb(0.10, 0.12, 0.18),
            1.08,
        );
        rect(
            commands,
            dx - 8.,
            233.,
            10.,
            4.,
            Color::srgb(0.30, 0.32, 0.38),
            1.08,
        );
    }

    // STORE interior - shelving rows
    for sy in [-210., -180., -150.] {
        rect(
            commands,
            -85.,
            sy,
            100.,
            8.,
            Color::srgb(0.45, 0.40, 0.32),
            1.06,
        );
        rect(
            commands,
            -85.,
            sy,
            96.,
            4.,
            Color::srgb(0.55, 0.50, 0.40),
            1.08,
        );
    }

    // LIBRARY interior - two columns of horizontal bookshelves with center aisle
    // Left stack column (near west wall)
    for sy in [268., 252., 236.] {
        rect(
            commands,
            -150.,
            sy,
            28.,
            10.,
            Color::srgb(0.28, 0.20, 0.10),
            1.06,
        );
        rect(
            commands,
            -150.,
            sy + 1.,
            26.,
            7.,
            Color::srgb(0.44, 0.32, 0.18),
            1.07,
        );
        rect(
            commands,
            -160.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.22, 0.38, 0.60),
            1.08,
        );
        rect(
            commands,
            -152.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.60, 0.22, 0.24),
            1.08,
        );
        rect(
            commands,
            -144.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.24, 0.58, 0.32),
            1.08,
        );
        rect(
            commands,
            -136.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.72, 0.62, 0.18),
            1.08,
        );
    }
    // Right stack column (near east wall)
    for sy in [268., 252., 236.] {
        rect(
            commands,
            -22.,
            sy,
            28.,
            10.,
            Color::srgb(0.28, 0.20, 0.10),
            1.06,
        );
        rect(
            commands,
            -22.,
            sy + 1.,
            26.,
            7.,
            Color::srgb(0.44, 0.32, 0.18),
            1.07,
        );
        rect(
            commands,
            -32.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.50, 0.26, 0.60),
            1.08,
        );
        rect(
            commands,
            -24.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.22, 0.50, 0.54),
            1.08,
        );
        rect(
            commands,
            -16.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.62, 0.38, 0.22),
            1.08,
        );
        rect(
            commands,
            -8.,
            sy + 1.,
            4.,
            7.,
            Color::srgb(0.36, 0.60, 0.26),
            1.08,
        );
    }

    // BANK interior - marble floor
    rect(
        commands,
        -425.,
        -180.,
        130.,
        140.,
        Color::srgb(0.78, 0.74, 0.64),
        1.03,
    );
    rect(
        commands,
        -425.,
        -180.,
        126.,
        136.,
        Color::srgb(0.82, 0.78, 0.68),
        1.04,
    );

    // GARAGE interior - concrete floor
    rect(
        commands,
        425.,
        -180.,
        130.,
        140.,
        Color::srgb(0.44, 0.42, 0.40),
        1.03,
    );

    // WELLNESS interior - exercise mats
    rect(
        commands,
        -285.,
        170.,
        36.,
        26.,
        Color::srgb(0.30, 0.55, 0.48),
        1.06,
    );
    rect(
        commands,
        -225.,
        170.,
        36.,
        26.,
        Color::srgb(0.30, 0.55, 0.48),
        1.06,
    );

    // CLINIC interior - tiles
    rect(
        commands,
        -255.,
        -180.,
        130.,
        140.,
        Color::srgb(0.88, 0.92, 0.90),
        1.03,
    );

    // CAFÉ interior - warm floor
    rect(
        commands,
        85.,
        -180.,
        130.,
        140.,
        Color::srgb(0.72, 0.58, 0.38),
        1.03,
    );

    // ADOPTION interior - warm purple
    rect(
        commands,
        255.,
        -180.,
        130.,
        140.,
        Color::srgb(0.70, 0.62, 0.82),
        1.03,
    );

    // -- Interactive objects -----------------------------------------------------

    // HOME - BED (-490, 250)
    obj(
        commands,
        -490.,
        250.,
        40.,
        20.,
        Color::srgb(0.30, 0.18, 0.09),
        ActionKind::Sleep,
        "[E] Sleep",
    );
    rect(
        commands,
        -490.,
        250.,
        36.,
        16.,
        Color::srgb(0.90, 0.87, 0.82),
        2.1,
    );
    rect(
        commands,
        -502.,
        253.,
        10.,
        6.,
        Color::srgb(0.96, 0.94, 0.92),
        2.2,
    );
    rect(
        commands,
        -490.,
        253.,
        10.,
        6.,
        Color::srgb(0.96, 0.94, 0.92),
        2.2,
    );
    rect(
        commands,
        -480.,
        246.,
        18.,
        8.,
        Color::srgb(0.48, 0.28, 0.65),
        2.15,
    );

    // HOME - FRIDGE (-350, 118)
    obj(
        commands,
        -350.,
        118.,
        20.,
        34.,
        Color::srgb(0.55, 0.58, 0.56),
        ActionKind::Eat,
        "[E] Eat",
    );
    rect(
        commands,
        -350.,
        126.,
        16.,
        18.,
        Color::srgb(0.82, 0.86, 0.84),
        2.1,
    );
    rect(
        commands,
        -350.,
        108.,
        16.,
        10.,
        Color::srgb(0.76, 0.80, 0.78),
        2.1,
    );
    rect(
        commands,
        -343.,
        126.,
        2.,
        12.,
        Color::srgb(0.48, 0.50, 0.54),
        2.2,
    );

    // HOME - SHOWER (-348, 252)
    obj(
        commands,
        -348.,
        252.,
        18.,
        24.,
        Color::srgb(0.45, 0.60, 0.72),
        ActionKind::Shower,
        "[E] Shower",
    );
    rect(
        commands,
        -348.,
        252.,
        14.,
        20.,
        Color::srgb(0.78, 0.88, 0.94),
        2.1,
    );
    rect(
        commands,
        -348.,
        260.,
        8.,
        2.,
        Color::srgb(0.50, 0.54, 0.60),
        2.2,
    );
    rect(
        commands,
        -348.,
        244.,
        4.,
        4.,
        Color::srgb(0.46, 0.50, 0.56),
        2.2,
    );

    // HOME - MEDITATION (-480, 120)
    obj(
        commands,
        -480.,
        120.,
        20.,
        20.,
        Color::srgb(0.48, 0.28, 0.18),
        ActionKind::Meditate,
        "[E] Meditate",
    );
    rect(
        commands,
        -480.,
        120.,
        16.,
        16.,
        Color::srgb(0.60, 0.38, 0.26),
        2.1,
    );
    rect(
        commands,
        -480.,
        120.,
        8.,
        8.,
        Color::srgb(0.70, 0.48, 0.34),
        2.2,
    );
    rect(
        commands,
        -480.,
        120.,
        3.,
        3.,
        Color::srgb(0.88, 0.68, 0.48),
        2.3,
    );

    // HOME - FREELANCE DESK (-430, 220)
    obj(
        commands,
        -430.,
        220.,
        34.,
        16.,
        Color::srgb(0.38, 0.26, 0.12),
        ActionKind::Freelance,
        "[E] Freelance Desk - work from home",
    );
    rect(
        commands,
        -430.,
        220.,
        30.,
        12.,
        Color::srgb(0.58, 0.44, 0.26),
        2.1,
    );
    rect(
        commands,
        -440.,
        223.,
        10.,
        8.,
        Color::srgb(0.10, 0.12, 0.18),
        2.2,
    );
    rect(
        commands,
        -440.,
        223.,
        8.,
        6.,
        Color::srgb(0.12, 0.36, 0.62),
        2.3,
    );
    rect(
        commands,
        -430.,
        218.,
        14.,
        4.,
        Color::srgb(0.20, 0.20, 0.24),
        2.2,
    );
    rect(
        commands,
        -420.,
        221.,
        4.,
        4.,
        Color::srgb(0.22, 0.22, 0.26),
        2.2,
    );

    // HOME - COFFEE (-490, 215)
    obj(
        commands,
        -490.,
        215.,
        14.,
        14.,
        Color::srgb(0.38, 0.28, 0.16),
        ActionKind::UseItem(ItemKind::Coffee),
        "[E] Drink Coffee",
    );
    rect(
        commands,
        -490.,
        219.,
        10.,
        7.,
        Color::srgb(0.20, 0.18, 0.20),
        2.1,
    );
    rect(
        commands,
        -490.,
        213.,
        10.,
        4.,
        Color::srgb(0.28, 0.26, 0.28),
        2.1,
    );
    rect(
        commands,
        -487.,
        210.,
        4.,
        4.,
        Color::srgb(0.88, 0.84, 0.78),
        2.2,
    );

    // HOME - VITAMINS (-490, 195)
    obj(
        commands,
        -490.,
        195.,
        14.,
        14.,
        Color::srgb(0.38, 0.28, 0.16),
        ActionKind::UseItem(ItemKind::Vitamins),
        "[E] Take Vitamins",
    );
    rect(
        commands,
        -490.,
        197.,
        6.,
        10.,
        Color::srgb(0.28, 0.66, 0.36),
        2.1,
    );
    rect(
        commands,
        -490.,
        201.,
        4.,
        3.,
        Color::srgb(0.20, 0.20, 0.22),
        2.2,
    );
    rect(
        commands,
        -490.,
        195.,
        4.,
        4.,
        Color::srgb(0.88, 0.88, 0.92),
        2.2,
    );

    // HOME - BOOKSHELF (-490, 170)
    obj(
        commands,
        -490.,
        170.,
        14.,
        14.,
        Color::srgb(0.35, 0.26, 0.14),
        ActionKind::UseItem(ItemKind::Books),
        "[E] Read Book",
    );
    rect(
        commands,
        -494.,
        170.,
        4.,
        10.,
        Color::srgb(0.72, 0.22, 0.22),
        2.1,
    );
    rect(
        commands,
        -490.,
        170.,
        4.,
        10.,
        Color::srgb(0.22, 0.42, 0.72),
        2.1,
    );
    rect(
        commands,
        -486.,
        170.,
        4.,
        10.,
        Color::srgb(0.28, 0.60, 0.28),
        2.1,
    );

    // HOME - EASEL (-390, 235)
    obj(
        commands,
        -390.,
        235.,
        16.,
        20.,
        Color::srgb(0.42, 0.30, 0.14),
        ActionKind::Hobby(HobbyKind::Painting),
        "[E] Paint (Painting skill)",
    );
    rect(
        commands,
        -390.,
        239.,
        10.,
        10.,
        Color::srgb(0.94, 0.92, 0.88),
        2.1,
    );
    rect(
        commands,
        -393.,
        241.,
        3.,
        3.,
        Color::srgb(0.84, 0.38, 0.26),
        2.2,
    );
    rect(
        commands,
        -388.,
        242.,
        3.,
        3.,
        Color::srgb(0.28, 0.52, 0.88),
        2.2,
    );
    rect(
        commands,
        -390.,
        227.,
        3.,
        7.,
        Color::srgb(0.35, 0.24, 0.10),
        2.1,
    );

    // HOME - GAMING (-390, 205)
    obj(
        commands,
        -390.,
        205.,
        16.,
        18.,
        Color::srgb(0.18, 0.22, 0.32),
        ActionKind::Hobby(HobbyKind::Gaming),
        "[E] Game (Gaming skill)",
    );
    rect(
        commands,
        -390.,
        208.,
        12.,
        8.,
        Color::srgb(0.08, 0.10, 0.14),
        2.1,
    );
    rect(
        commands,
        -390.,
        208.,
        8.,
        6.,
        Color::srgb(0.10, 0.32, 0.58),
        2.2,
    );
    rect(
        commands,
        -390.,
        200.,
        8.,
        4.,
        Color::srgb(0.30, 0.30, 0.38),
        2.2,
    );

    // HOME - PIANO (-390, 175)
    obj(
        commands,
        -390.,
        175.,
        16.,
        18.,
        Color::srgb(0.48, 0.30, 0.14),
        ActionKind::Hobby(HobbyKind::Music),
        "[E] Play Music (Music skill)",
    );
    rect(
        commands,
        -390.,
        178.,
        12.,
        5.,
        Color::srgb(0.92, 0.90, 0.86),
        2.1,
    );
    for bk in [-394., -390., -386.] {
        rect(
            commands,
            bk,
            179.,
            2.,
            3.,
            Color::srgb(0.12, 0.10, 0.10),
            2.2,
        );
    }
    rect(
        commands,
        -390.,
        169.,
        10.,
        6.,
        Color::srgb(0.42, 0.26, 0.10),
        2.1,
    );

    // HOME - PET BOWL (-425, 115)
    obj(
        commands,
        -425.,
        115.,
        14.,
        14.,
        Color::srgb(0.50, 0.36, 0.18),
        ActionKind::FeedPet,
        "[E] Pet Bowl - Feed/Adopt pet ($5 feed / $50 adopt)",
    );
    rect(
        commands,
        -425.,
        115.,
        10.,
        10.,
        Color::srgb(0.52, 0.58, 0.66),
        2.1,
    );
    rect(
        commands,
        -425.,
        115.,
        6.,
        6.,
        Color::srgb(0.28, 0.52, 0.82),
        2.2,
    );

    // HOME - CRAFT STATION (-396, 130)
    obj(
        commands,
        -396.,
        130.,
        20.,
        16.,
        Color::srgb(0.32, 0.40, 0.28),
        ActionKind::Craft,
        "[E] Craft Station [1]Cook [2]GiftBox [3]Smoothie",
    );
    rect(
        commands,
        -396.,
        132.,
        16.,
        10.,
        Color::srgb(0.44, 0.52, 0.38),
        2.1,
    );
    rect(
        commands,
        -400.,
        134.,
        5.,
        5.,
        Color::srgb(0.72, 0.42, 0.18),
        2.2,
    );
    rect(
        commands,
        -392.,
        134.,
        5.,
        5.,
        Color::srgb(0.22, 0.62, 0.32),
        2.2,
    );

    // HOME - PARTY CORNER (-395, 270)
    obj(
        commands,
        -395.,
        270.,
        20.,
        20.,
        Color::srgb(0.58, 0.20, 0.36),
        ActionKind::ThrowParty,
        "[E] Party Corner - Throw a party! ($40)",
    );
    rect(
        commands,
        -395.,
        272.,
        14.,
        12.,
        Color::srgb(0.70, 0.30, 0.48),
        2.1,
    );
    rect(
        commands,
        -395.,
        275.,
        7.,
        6.,
        Color::srgb(0.95, 0.88, 0.35),
        2.2,
    );
    rect(
        commands,
        -395.,
        278.,
        2.,
        4.,
        Color::srgb(0.95, 0.88, 0.35),
        2.3,
    );
    rect(
        commands,
        -395.,
        280.,
        3.,
        2.,
        Color::srgb(0.95, 0.55, 0.20),
        2.3,
    );
    rect(
        commands,
        -401.,
        265.,
        3.,
        3.,
        Color::srgb(0.95, 0.38, 0.38),
        2.1,
    );
    rect(
        commands,
        -389.,
        266.,
        3.,
        3.,
        Color::srgb(0.38, 0.72, 0.95),
        2.1,
    );

    // OFFICE - WORK DESK (455, 160)
    obj(
        commands,
        455.,
        160.,
        44.,
        24.,
        Color::srgb(0.32, 0.22, 0.10),
        ActionKind::Work,
        "[E] Work",
    );
    rect(
        commands,
        455.,
        160.,
        40.,
        20.,
        Color::srgb(0.46, 0.34, 0.18),
        2.1,
    );
    rect(
        commands,
        443.,
        163.,
        12.,
        10.,
        Color::srgb(0.08, 0.10, 0.16),
        2.2,
    );
    rect(
        commands,
        443.,
        163.,
        9.,
        7.,
        Color::srgb(0.12, 0.38, 0.68),
        2.3,
    );
    rect(
        commands,
        443.,
        156.,
        4.,
        3.,
        Color::srgb(0.36, 0.36, 0.40),
        2.2,
    );
    rect(
        commands,
        457.,
        160.,
        16.,
        4.,
        Color::srgb(0.18, 0.18, 0.22),
        2.2,
    );
    rect(
        commands,
        469.,
        164.,
        8.,
        6.,
        Color::srgb(0.88, 0.86, 0.82),
        2.2,
    );
    rect(
        commands,
        469.,
        155.,
        4.,
        5.,
        Color::srgb(0.55, 0.35, 0.20),
        2.2,
    );

    // STORE - SHOP COUNTER (-85, -180)
    obj(
        commands,
        -85.,
        -180.,
        56.,
        22.,
        Color::srgb(0.60, 0.52, 0.22),
        ActionKind::Shop,
        "[E] Shop",
    );
    rect(
        commands,
        -85.,
        -177.,
        50.,
        12.,
        Color::srgb(0.80, 0.72, 0.38),
        2.1,
    );
    rect(
        commands,
        -103.,
        -180.,
        10.,
        10.,
        Color::srgb(0.22, 0.22, 0.28),
        2.2,
    );
    rect(
        commands,
        -103.,
        -177.,
        8.,
        4.,
        Color::srgb(0.32, 0.32, 0.38),
        2.3,
    );
    rect(
        commands,
        -71.,
        -181.,
        7.,
        8.,
        Color::srgb(0.88, 0.80, 0.58),
        2.2,
    );
    rect(
        commands,
        -63.,
        -181.,
        7.,
        8.,
        Color::srgb(0.58, 0.80, 0.55),
        2.2,
    );

    // PARK - SHELTER (40, 245) rough sleeping
    obj(
        commands,
        40.,
        245.,
        36.,
        18.,
        Color::srgb(0.45, 0.35, 0.25),
        ActionKind::SleepRough,
        "[E] Sleep here (rough rest)",
    );
    rect(
        commands,
        40.,
        247.,
        30.,
        6.,
        Color::srgb(0.60, 0.50, 0.38),
        2.1,
    );
    rect(
        commands,
        40.,
        252.,
        32.,
        3.,
        Color::srgb(0.38, 0.28, 0.18),
        2.2,
    );

    // PARK - BENCH (85, 160) relax
    obj(
        commands,
        85.,
        160.,
        40.,
        16.,
        Color::srgb(0.32, 0.20, 0.08),
        ActionKind::Relax,
        "[E] Relax",
    );
    rect(
        commands,
        85.,
        163.,
        34.,
        4.,
        Color::srgb(0.50, 0.32, 0.14),
        2.1,
    );
    rect(
        commands,
        85.,
        157.,
        34.,
        4.,
        Color::srgb(0.44, 0.28, 0.11),
        2.1,
    );
    rect(
        commands,
        70.,
        159.,
        3.,
        12.,
        Color::srgb(0.32, 0.20, 0.08),
        2.1,
    );
    rect(
        commands,
        100.,
        159.,
        3.,
        12.,
        Color::srgb(0.32, 0.20, 0.08),
        2.1,
    );

    // PARK - PULL-UP BAR (130, 140) exercise
    obj(
        commands,
        130.,
        140.,
        20.,
        34.,
        Color::srgb(0.22, 0.40, 0.20),
        ActionKind::Exercise,
        "[E] Exercise",
    );
    rect(
        commands,
        121.,
        146.,
        3.,
        16.,
        Color::srgb(0.46, 0.50, 0.54),
        2.1,
    );
    rect(
        commands,
        139.,
        146.,
        3.,
        16.,
        Color::srgb(0.46, 0.50, 0.54),
        2.1,
    );
    rect(
        commands,
        130.,
        154.,
        18.,
        3.,
        Color::srgb(0.52, 0.56, 0.60),
        2.2,
    );
    rect(
        commands,
        130.,
        134.,
        14.,
        5.,
        Color::srgb(0.36, 0.40, 0.44),
        2.2,
    );

    // BANK - TELLER COUNTER (-425, -180)
    obj(
        commands,
        -425.,
        -180.,
        36.,
        24.,
        Color::srgb(0.62, 0.52, 0.20),
        ActionKind::Bank,
        "[E] Bank  [1-8] actions",
    );
    rect(
        commands,
        -425.,
        -175.,
        32.,
        12.,
        Color::srgb(0.84, 0.76, 0.54),
        2.1,
    );
    rect(
        commands,
        -425.,
        -180.,
        20.,
        14.,
        Color::srgba(0.68, 0.86, 0.94, 0.55),
        2.2,
    );
    rect(
        commands,
        -425.,
        -187.,
        18.,
        4.,
        Color::srgb(0.42, 0.34, 0.12),
        2.2,
    );

    // LIBRARY - READING DESK (-60, 160)
    obj(
        commands,
        -60.,
        160.,
        40.,
        24.,
        Color::srgb(0.34, 0.24, 0.12),
        ActionKind::StudyCourse,
        "[E] Study - $30 + 20 energy -> +0.5 random skill",
    );
    rect(
        commands,
        -60.,
        162.,
        36.,
        18.,
        Color::srgb(0.48, 0.36, 0.20),
        2.1,
    );
    rect(
        commands,
        -60.,
        163.,
        16.,
        10.,
        Color::srgb(0.92, 0.90, 0.84),
        2.2,
    );
    rect(
        commands,
        -85.,
        183.,
        2.,
        10.,
        Color::srgb(0.40, 0.30, 0.16),
        2.3,
    );
    rect(
        commands,
        -75.,
        184.,
        4.,
        7.,
        Color::srgb(0.85, 0.72, 0.28),
        2.2,
    );

    // GARAGE - WORKBENCH (425, -180)
    obj(
        commands,
        425.,
        -180.,
        40.,
        24.,
        Color::srgb(0.38, 0.36, 0.44),
        ActionKind::BuyTransport,
        "[E] Transport  [1] Bike $80sav  [2] Car $300sav",
    );
    rect(
        commands,
        425.,
        -178.,
        36.,
        16.,
        Color::srgb(0.50, 0.48, 0.56),
        2.1,
    );
    rect(
        commands,
        413.,
        -184.,
        10.,
        8.,
        Color::srgb(0.16, 0.16, 0.18),
        2.2,
    );
    rect(
        commands,
        413.,
        -184.,
        6.,
        6.,
        Color::srgb(0.28, 0.28, 0.32),
        2.3,
    );
    rect(
        commands,
        431.,
        -179.,
        12.,
        3.,
        Color::srgb(0.52, 0.54, 0.58),
        2.2,
    );
    rect(
        commands,
        437.,
        -178.,
        4.,
        6.,
        Color::srgb(0.52, 0.54, 0.58),
        2.2,
    );

    // WELLNESS - TREADMILL (-290, 210)
    obj(
        commands,
        -290.,
        210.,
        38.,
        24.,
        Color::srgb(0.18, 0.38, 0.60),
        ActionKind::GymSession,
        "[E] Gym - $5 fee, +Health/Fitness (better than park)",
    );
    rect(
        commands,
        -290.,
        210.,
        34.,
        18.,
        Color::srgb(0.16, 0.16, 0.20),
        2.1,
    );
    rect(
        commands,
        -290.,
        208.,
        28.,
        8.,
        Color::srgb(0.26, 0.26, 0.30),
        2.2,
    );
    rect(
        commands,
        -301.,
        214.,
        3.,
        10.,
        Color::srgb(0.48, 0.50, 0.56),
        2.2,
    );
    rect(
        commands,
        -279.,
        214.,
        3.,
        10.,
        Color::srgb(0.48, 0.50, 0.56),
        2.2,
    );
    rect(
        commands,
        -290.,
        218.,
        8.,
        4.,
        Color::srgb(0.16, 0.48, 0.72),
        2.3,
    );

    // WELLNESS - CAFÉ COUNTER (-220, 210)
    obj(
        commands,
        -220.,
        210.,
        38.,
        20.,
        Color::srgb(0.58, 0.38, 0.14),
        ActionKind::Cafe,
        "[E] Café - $12, +25 Energy +12 Mood",
    );
    rect(
        commands,
        -220.,
        213.,
        34.,
        12.,
        Color::srgb(0.72, 0.52, 0.24),
        2.1,
    );
    rect(
        commands,
        -230.,
        210.,
        8.,
        12.,
        Color::srgb(0.20, 0.18, 0.20),
        2.2,
    );
    rect(
        commands,
        -230.,
        214.,
        6.,
        3.,
        Color::srgb(0.30, 0.28, 0.32),
        2.3,
    );
    rect(
        commands,
        -214.,
        209.,
        4.,
        4.,
        Color::srgb(0.90, 0.86, 0.80),
        2.2,
    );
    rect(
        commands,
        -208.,
        209.,
        4.,
        4.,
        Color::srgb(0.90, 0.86, 0.80),
        2.2,
    );

    // WELLNESS - CLINIC BED (-255, 140)
    obj(
        commands,
        -255.,
        140.,
        36.,
        20.,
        Color::srgb(0.60, 0.76, 0.72),
        ActionKind::Clinic,
        "[E] Clinic - $40, restore +35 Health",
    );
    rect(
        commands,
        -255.,
        140.,
        32.,
        16.,
        Color::srgb(0.88, 0.94, 0.92),
        2.1,
    );
    rect(
        commands,
        -267.,
        143.,
        8.,
        5.,
        Color::srgb(0.96, 0.97, 0.96),
        2.2,
    );
    rect(
        commands,
        -253.,
        137.,
        16.,
        8.,
        Color::srgb(0.64, 0.82, 0.90),
        2.2,
    );
    rect(
        commands,
        -255.,
        143.,
        8.,
        3.,
        Color::srgb(0.90, 0.18, 0.26),
        2.3,
    );
    rect(
        commands,
        -255.,
        140.,
        3.,
        8.,
        Color::srgb(0.90, 0.18, 0.26),
        2.3,
    );

    // CAFÉ - COUNTER (85, -180)
    obj(
        commands,
        85.,
        -180.,
        40.,
        20.,
        Color::srgb(0.58, 0.38, 0.14),
        ActionKind::Cafe,
        "[E] Café - $12, +25 Energy +12 Mood",
    );
    rect(
        commands,
        85.,
        -177.,
        36.,
        12.,
        Color::srgb(0.72, 0.52, 0.24),
        2.1,
    );
    rect(
        commands,
        75.,
        -180.,
        8.,
        12.,
        Color::srgb(0.20, 0.18, 0.20),
        2.2,
    );
    rect(
        commands,
        93.,
        -182.,
        4.,
        4.,
        Color::srgb(0.90, 0.86, 0.80),
        2.2,
    );

    // CLINIC - BED (-255, -180)
    obj(
        commands,
        -255.,
        -180.,
        36.,
        20.,
        Color::srgb(0.60, 0.76, 0.72),
        ActionKind::Clinic,
        "[E] Clinic - $40, restore +35 Health",
    );
    rect(
        commands,
        -255.,
        -180.,
        32.,
        16.,
        Color::srgb(0.88, 0.94, 0.92),
        2.1,
    );
    rect(
        commands,
        -267.,
        -177.,
        8.,
        5.,
        Color::srgb(0.96, 0.97, 0.96),
        2.2,
    );
    rect(
        commands,
        -255.,
        -185.,
        14.,
        3.,
        Color::srgb(0.90, 0.18, 0.26),
        2.3,
    );
    rect(
        commands,
        -255.,
        -182.,
        3.,
        8.,
        Color::srgb(0.90, 0.18, 0.26),
        2.3,
    );

    // ADOPTION - three stations
    obj(
        commands,
        220.,
        -170.,
        24.,
        18.,
        Color::srgb(0.70, 0.56, 0.88),
        ActionKind::AdoptPet(PetKind::Cat),
        "[E] Adopt Cat - $300",
    );
    rect(
        commands,
        220.,
        -170.,
        18.,
        12.,
        Color::srgb(0.84, 0.70, 0.96),
        2.1,
    );
    rect(
        commands,
        220.,
        -165.,
        7.,
        5.,
        Color::srgb(0.92, 0.82, 0.98),
        2.2,
    );

    obj(
        commands,
        255.,
        -170.,
        24.,
        18.,
        Color::srgb(0.60, 0.44, 0.78),
        ActionKind::AdoptPet(PetKind::Dog),
        "[E] Adopt Dog - $300",
    );
    rect(
        commands,
        255.,
        -170.,
        18.,
        12.,
        Color::srgb(0.76, 0.62, 0.90),
        2.1,
    );
    rect(
        commands,
        255.,
        -165.,
        8.,
        6.,
        Color::srgb(0.88, 0.76, 0.96),
        2.2,
    );

    obj(
        commands,
        290.,
        -200.,
        20.,
        16.,
        Color::srgb(0.50, 0.70, 0.88),
        ActionKind::AdoptPet(PetKind::Fish),
        "[E] Adopt Fish - $300",
    );
    rect(
        commands,
        290.,
        -200.,
        16.,
        10.,
        Color::srgb(0.62, 0.82, 0.96),
        2.1,
    );
    rect(
        commands,
        290.,
        -198.,
        6.,
        5.,
        Color::srgba(0.35, 0.65, 0.92, 0.60),
        2.2,
    );

    // -- Extra collective-building objects (3 additional per building) ----------

    // OFFICE (425, 180): three back-row work desks (organized line at y=235)
    obj(
        commands,
        385.,
        235.,
        30.,
        16.,
        Color::srgb(0.32, 0.22, 0.10),
        ActionKind::Work,
        "[E] Work (desk 2)",
    );
    obj(
        commands,
        425.,
        235.,
        30.,
        16.,
        Color::srgb(0.32, 0.22, 0.10),
        ActionKind::Work,
        "[E] Work (desk 3)",
    );
    obj(
        commands,
        465.,
        235.,
        30.,
        16.,
        Color::srgb(0.32, 0.22, 0.10),
        ActionKind::Work,
        "[E] Work (desk 4)",
    );

    // LIBRARY (-85, 180): computer terminal, media room, tutoring desk
    obj(
        commands,
        -120.,
        220.,
        28.,
        18.,
        Color::srgb(0.18, 0.28, 0.44),
        ActionKind::ComputerLab,
        "[E] Computer Lab — browse / research",
    );
    obj(
        commands,
        -50.,
        220.,
        28.,
        18.,
        Color::srgb(0.30, 0.28, 0.48),
        ActionKind::Relax,
        "[E] Media Room — chill & watch",
    );
    obj(
        commands,
        -85.,
        135.,
        30.,
        18.,
        Color::srgb(0.34, 0.24, 0.12),
        ActionKind::StudyCourse,
        "[E] Tutoring — $30 study session",
    );
    obj(
        commands,
        -55.,
        180.,
        24.,
        14.,
        Color::srgb(0.62, 0.58, 0.42),
        ActionKind::PrintShop,
        "[E] Print Shop — $5 per page",
    );

    // WELLNESS (-255, 180): yoga mat, sauna, pharmacy counter
    obj(
        commands,
        -290.,
        135.,
        28.,
        18.,
        Color::srgb(0.38, 0.60, 0.38),
        ActionKind::GymSession,
        "[E] Yoga Mat — $5 fitness session",
    );
    obj(
        commands,
        -220.,
        135.,
        22.,
        22.,
        Color::srgb(0.72, 0.44, 0.22),
        ActionKind::Relax,
        "[E] Sauna — relax & destress",
    );
    obj(
        commands,
        -255.,
        175.,
        24.,
        14.,
        Color::srgb(0.28, 0.66, 0.36),
        ActionKind::UseItem(ItemKind::Vitamins),
        "[E] Pharmacy Counter",
    );

    // STORE (-85, -180): deli counter, bulk goods, pharmacy aisle
    obj(
        commands,
        -120.,
        -140.,
        28.,
        16.,
        Color::srgb(0.72, 0.52, 0.22),
        ActionKind::Shop,
        "[E] Deli Counter [1-4]",
    );
    obj(
        commands,
        -50.,
        -140.,
        28.,
        16.,
        Color::srgb(0.58, 0.72, 0.36),
        ActionKind::Shop,
        "[E] Bulk Goods [1-4]",
    );
    obj(
        commands,
        -85.,
        -218.,
        28.,
        16.,
        Color::srgb(0.34, 0.60, 0.38),
        ActionKind::UseItem(ItemKind::Vitamins),
        "[E] Pharmacy Aisle",
    );

    // CAFÉ (85, -180): patio seat, barista bar, pastry display
    obj(
        commands,
        55.,
        -140.,
        28.,
        14.,
        Color::srgb(0.60, 0.38, 0.14),
        ActionKind::Relax,
        "[E] Patio Seat — relax outdoors",
    );
    obj(
        commands,
        120.,
        -140.,
        28.,
        14.,
        Color::srgb(0.52, 0.34, 0.12),
        ActionKind::Cafe,
        "[E] Barista Bar — $12 order",
    );
    obj(
        commands,
        85.,
        -218.,
        24.,
        14.,
        Color::srgb(0.86, 0.78, 0.52),
        ActionKind::UseItem(ItemKind::Coffee),
        "[E] Pastry Display",
    );

    // BANK (-425, -180): ATM, loan officer, investment advisor
    obj(
        commands,
        -465.,
        -135.,
        18.,
        22.,
        Color::srgb(0.52, 0.44, 0.18),
        ActionKind::Bank,
        "[E] ATM  [1]Dep [2]Wth",
    );
    obj(
        commands,
        -385.,
        -135.,
        24.,
        18.,
        Color::srgb(0.62, 0.52, 0.20),
        ActionKind::Bank,
        "[E] Loan Officer  [4]Loan [5]Repay",
    );
    obj(
        commands,
        -425.,
        -220.,
        28.,
        18.,
        Color::srgb(0.48, 0.44, 0.20),
        ActionKind::Bank,
        "[E] Advisor  [6-8] Invest",
    );

    // CLINIC (-255, -180): dental chair, eye exam, pharmacy window
    obj(
        commands,
        -295.,
        -140.,
        28.,
        18.,
        Color::srgb(0.60, 0.80, 0.76),
        ActionKind::DentalVisit,
        "[E] Dental Chair",
    );
    obj(
        commands,
        -215.,
        -140.,
        28.,
        18.,
        Color::srgb(0.58, 0.72, 0.88),
        ActionKind::EyeExam,
        "[E] Eye Exam Station",
    );
    obj(
        commands,
        -255.,
        -218.,
        26.,
        16.,
        Color::srgb(0.28, 0.66, 0.44),
        ActionKind::UseItem(ItemKind::Vitamins),
        "[E] Pharmacy Window",
    );

    // GARAGE (425, -180): gas pump, service bay, parts shelf
    obj(
        commands,
        395.,
        -140.,
        20.,
        30.,
        Color::srgb(0.44, 0.44, 0.50),
        ActionKind::GasUp,
        "[E] Gas Pump — fill up",
    );
    obj(
        commands,
        455.,
        -140.,
        30.,
        26.,
        Color::srgb(0.36, 0.34, 0.44),
        ActionKind::RepairVehicle,
        "[E] Service Bay — repair vehicle",
    );
    obj(
        commands,
        425.,
        -220.,
        28.,
        16.,
        Color::srgb(0.48, 0.46, 0.54),
        ActionKind::Shop,
        "[E] Parts Shelf [1-4]",
    );

    // PARK (85, 180): sports court, playground, food cart (additional content)
    obj(
        commands,
        115.,
        215.,
        32.,
        20.,
        Color::srgb(0.38, 0.54, 0.38),
        ActionKind::Exercise,
        "[E] Sports Court — Exercise",
    );
    obj(
        commands,
        55.,
        215.,
        30.,
        20.,
        Color::srgb(0.62, 0.48, 0.26),
        ActionKind::Relax,
        "[E] Playground — kids area, Relax",
    );
    obj(
        commands,
        85.,
        125.,
        24.,
        18.,
        Color::srgb(0.70, 0.50, 0.20),
        ActionKind::Cafe,
        "[E] Food Cart — $12 snack",
    );

    // ADOPTION (255, -180): training area, vet check (in addition to adopt stations)
    obj(
        commands,
        220.,
        -210.,
        24.,
        16.,
        Color::srgb(0.38, 0.58, 0.38),
        ActionKind::Exercise,
        "[E] Training Area — exercise with pet",
    );
    obj(
        commands,
        290.,
        -165.,
        22.,
        16.,
        Color::srgb(0.60, 0.76, 0.72),
        ActionKind::Clinic,
        "[E] Vet Check — $40 pet health",
    );

    // -- Trees (inside PARK zone) -----------------------------------------------
    for (x, y, s) in [
        (40., 250., 14.),
        (130., 250., 14.),
        (30., 135., 12.),
        (140., 135., 12.),
        (85., 230., 12.),
    ] {
        rect(
            commands,
            x + 3.,
            y - s * 0.62,
            s * 1.0,
            s * 0.34,
            Color::srgba(0., 0., 0., 0.28),
            2.9,
        );
        rect(
            commands,
            x,
            y - s * 0.5 + 3.,
            s * 0.35,
            6.,
            Color::srgb(0.32, 0.20, 0.08),
            2.95,
        );
        rect(commands, x, y, s, s, Color::srgb(0.12, 0.40, 0.12), 3.0);
        let hs = s * 0.65;
        rect(
            commands,
            x - s * 0.09,
            y + s * 0.07,
            hs,
            hs,
            Color::srgb(0.20, 0.58, 0.20),
            3.05,
        );
        let ss = s * 0.30;
        rect(
            commands,
            x - s * 0.20,
            y + s * 0.18,
            ss,
            ss,
            Color::srgb(0.36, 0.74, 0.28),
            3.1,
        );
    }
}

fn spawn_npcs(commands: &mut Commands) {
    // NPC zone constants (pre-scale coords × S = world coords)
    let _home_z = Vec2::new(-425. * S, 180. * S);
    let office_z = Vec2::new(425. * S, 180. * S);
    let park_z = Vec2::new(85. * S, 180. * S);
    let store_z = Vec2::new(-85. * S, -180. * S);
    let library_z = Vec2::new(-85. * S, 180. * S);
    let garage_z = Vec2::new(425. * S, -180. * S);

    spawn_npc(
        commands,
        "Alex",
        0,
        Color::srgb(0.80, 0.22, 0.22),
        Color::srgb(0.50, 0.12, 0.12),
        Color::srgb(0.94, 0.80, 0.65),
        Color::srgb(0.34, 0.20, 0.08),
        park_z,
        100. * S,
        1,
        NpcPersonality::Neutral,
        Vec2::new(-425. * S, 180. * S),
        office_z,
    );
    spawn_npc(
        commands,
        "Sam",
        1,
        Color::srgb(0.22, 0.68, 0.32),
        Color::srgb(0.12, 0.42, 0.18),
        Color::srgb(0.76, 0.58, 0.40),
        Color::srgb(0.10, 0.08, 0.06),
        park_z,
        100. * S,
        2,
        NpcPersonality::Neutral,
        Vec2::new(-425. * S, -180. * S),
        office_z,
    );
    spawn_npc(
        commands,
        "Mia",
        2,
        Color::srgb(0.58, 0.32, 0.88),
        Color::srgb(0.35, 0.16, 0.58),
        Color::srgb(0.96, 0.84, 0.70),
        Color::srgb(0.66, 0.20, 0.10),
        store_z,
        60. * S,
        3,
        NpcPersonality::Neutral,
        Vec2::new(-255. * S, -180. * S),
        store_z,
    );
    spawn_npc(
        commands,
        "Jordan",
        3,
        Color::srgb(0.95, 0.70, 0.10),
        Color::srgb(0.65, 0.45, 0.05),
        Color::srgb(0.88, 0.68, 0.50),
        Color::srgb(0.55, 0.30, 0.08),
        park_z,
        100. * S,
        4,
        NpcPersonality::Cheerful,
        store_z,
        store_z,
    );
    spawn_npc(
        commands,
        "Taylor",
        4,
        Color::srgb(0.20, 0.45, 0.80),
        Color::srgb(0.10, 0.25, 0.52),
        Color::srgb(0.95, 0.88, 0.78),
        Color::srgb(0.78, 0.72, 0.65),
        library_z,
        60. * S,
        5,
        NpcPersonality::Wise,
        library_z,
        library_z,
    );
    spawn_npc(
        commands,
        "Casey",
        5,
        Color::srgb(0.10, 0.68, 0.62),
        Color::srgb(0.05, 0.40, 0.36),
        Color::srgb(0.72, 0.52, 0.36),
        Color::srgb(0.08, 0.06, 0.04),
        garage_z,
        80. * S,
        6,
        NpcPersonality::Influential,
        office_z,
        office_z,
    );
}

fn spawn_player_entity(commands: &mut Commands) {
    commands
        .spawn((
            Transform::from_xyz(0., 0., 10.),
            Visibility::default(),
            Player,
            LocalPlayer,
            PlayerId(0),
            PlayerMovement::default(),
            VehicleState::default(),
            BankInput::default(),
            ActionPrompt::default(),
            PlayerStats::default(),
            Inventory::default(),
            Skills::default(),
            WorkStreak::default(),
            HousingTier::default(),
            Furnishings::default(),
        ))
        .with_children(|p| {
            spawn_human(
                p,
                Color::srgb(0.90, 0.52, 0.12),
                Color::srgb(0.58, 0.28, 0.06),
                Color::srgb(0.94, 0.80, 0.65),
                Color::srgb(0.36, 0.22, 0.09),
            );
            p.spawn((
                Sprite {
                    color: Color::srgb(1., 1., 0.55),
                    custom_size: Some(Vec2::splat(5. * S)),
                    ..default()
                },
                Transform::from_xyz(0., 18. * S, 3.5),
                PlayerIndicator,
            ));
        });

    // Day/night ambient overlay
    commands.spawn((
        Sprite {
            color: Color::srgba(0., 0., 0.12, 0.),
            custom_size: Some(Vec2::splat(24000.)),
            ..default()
        },
        Transform::from_xyz(0., 0., 50.),
        DayNightOverlay,
    ));
    // Interactable proximity highlight
    commands.spawn((
        Sprite {
            color: Color::srgba(1., 1., 0.5, 0.),
            custom_size: Some(Vec2::splat(30. * S)),
            ..default()
        },
        Transform::from_xyz(0., 0., 1.98),
        InteractHighlight,
    ));
}

fn spawn_collision_walls_and_roads(commands: &mut Commands) {
    // -- Collision walls --------------------------------------------------------

    // World boundary
    wall(commands, 0., 560., 1200., 20.); // north (extended for back street)
    wall(commands, 0., -330., 1200., 20.); // south
    wall(commands, -700., 75., 20., 830.); // west
    wall(commands, 700., 75., 20., 830.); // east

    // Pond obstacle (park)
    wall(commands, 55., 215., 44., 34.);

    // Tree obstacles (park canopy footprints)
    for (tx, ty, ts) in [
        (40., 250., 14.),
        (130., 250., 14.),
        (30., 135., 12.),
        (140., 135., 12.),
        (85., 230., 12.),
    ] {
        wall(commands, tx, ty, ts * 0.75, ts * 0.75);
    }

    // -- Back road (north, y=290) road markings and lamp posts ------------------
    // (Road surface and sidewalk colours are handled by the tilemap)
    // Back road edge lines
    rect(
        commands,
        0.,
        357.,
        3000.,
        2.,
        Color::srgba(1., 1., 0.8, 0.10),
        0.6,
    );
    rect(
        commands,
        0.,
        303.,
        3000.,
        2.,
        Color::srgba(1., 1., 0.8, 0.10),
        0.6,
    );
    // Back road centre dashes
    for i in -17i32..=17 {
        let x = i as f32 * 40.;
        rect(
            commands,
            x,
            330.,
            18.,
            3.,
            Color::srgba(1., 1., 0.75, 0.20),
            0.7,
        );
    }
    // Back street lamp posts
    for &(lx, ly) in &[
        (-340., 305.),
        (-170., 305.),
        (0., 305.),
        (170., 305.),
        (340., 305.),
    ] {
        lamp_post(commands, lx, ly);
    }

    // -- Side alley lamp posts --------------------------------------------------
    // (Alley pavement colours are handled by the tilemap)
    for &ly in &[-200., -80., 80., 200., 320.] {
        lamp_post(commands, -610., ly);
    }
    for &ly in &[-200., -80., 80., 200., 320.] {
        lamp_post(commands, 610., ly);
    }

    // APARTMENTS zone label (floor colour handled by tilemap)
    zone_label(commands, 0., 460., 160., "APARTMENTS");
    commands.spawn(Building {
        name: "APARTMENTS",
        kind: BuildingKind::Collective,
    });
    // 6 apartment unit objects
    for (i, ux) in [-190., -110., -30., 50., 130., 210.].iter().enumerate() {
        let uid = i as u32 + 1;
        commands.spawn((
            Sprite {
                color: Color::srgba(0., 0., 0., 0.45),
                custom_size: Some(Vec2::new(52. * S, 44. * S)),
                ..default()
            },
            Transform::from_xyz((ux + 2.) * S, (456. - 2.) * S, 1.95),
        ));
        commands.spawn((
            Sprite {
                color: Color::srgb(0.80, 0.72, 0.90),
                custom_size: Some(Vec2::new(48. * S, 40. * S)),
                ..default()
            },
            Transform::from_xyz(ux * S, 456. * S, 2.),
            Interactable {
                action: ActionKind::RentUnit(uid),
                prompt: format!("[E] Rent Apt {}", uid),
            },
            ObjectSize(Vec2::new(48. * S, 40. * S)),
            ApartmentUnit {
                unit_id: uid,
                owner: None,
            },
        ));
    }
    // APARTMENTS building walls
    let ac = Color::srgb(0.45, 0.38, 0.60);
    vis_wall(commands, 0., 540., 500., 10., ac); // north
    vis_wall(commands, -250., 460., 10., 160., ac); // west
    vis_wall(commands, 250., 460., 10., 160., ac); // east
    vis_wall(commands, -100., 380., 200., 10., ac); // south-left
    vis_wall(commands, 100., 380., 200., 10., ac); // south-right
    // doorway gap is at x=0 ± 50 (100px wide)

    // SCHOOL building walls (design center -450, 460; size 150x160)
    let sc = Color::srgb(0.55, 0.22, 0.18);
    vis_wall(commands, -450., 540., 150., 10., sc); // north
    vis_wall(commands, -525., 460., 10., 160., sc); // west
    vis_wall(commands, -375., 460., 10., 160., sc); // east
    vis_wall(commands, -495., 380., 60., 10., sc); // south-left  (-525..-465)
    vis_wall(commands, -405., 380., 60., 10., sc); // south-right (-435..-375)
    // doorway gap is at x=-450 ± 15 (30px wide)
    // Decorative: white classroom windows
    rect(
        commands,
        -490.,
        500.,
        18.,
        18.,
        Color::srgb(0.86, 0.94, 0.98),
        1.5,
    );
    rect(
        commands,
        -450.,
        500.,
        18.,
        18.,
        Color::srgb(0.86, 0.94, 0.98),
        1.5,
    );
    rect(
        commands,
        -410.,
        500.,
        18.,
        18.,
        Color::srgb(0.86, 0.94, 0.98),
        1.5,
    );
    // Flagpole + flag
    rect(
        commands,
        -380.,
        555.,
        2.,
        50.,
        Color::srgb(0.20, 0.20, 0.20),
        1.55,
    );
    rect(
        commands,
        -370.,
        590.,
        18.,
        12.,
        Color::srgb(0.85, 0.20, 0.18),
        1.6,
    );
    // "Blackboard" interior strip
    rect(
        commands,
        -450.,
        470.,
        90.,
        6.,
        Color::srgb(0.10, 0.16, 0.10),
        1.7,
    );
    // SCHOOL interactables (interior, accessible via south door at x=-450)
    obj(
        commands,
        -480.,
        440.,
        22.,
        18.,
        Color::srgb(0.42, 0.28, 0.18),
        ActionKind::StudyCourse,
        "[E] Attend Class",
    );
    obj(
        commands,
        -420.,
        440.,
        22.,
        18.,
        Color::srgb(0.30, 0.42, 0.58),
        ActionKind::ComputerLab,
        "[E] Computer Lab",
    );
    obj(
        commands,
        -450.,
        490.,
        24.,
        14.,
        Color::srgb(0.55, 0.40, 0.22),
        ActionKind::Hobby(HobbyKind::Painting),
        "[E] Art Class",
    );

    // TRANSIT station walls (design center 450, 460; size 150x160)
    let tc = Color::srgb(0.32, 0.36, 0.42);
    vis_wall(commands, 450., 540., 150., 10., tc); // north (back wall)
    vis_wall(commands, 375., 460., 10., 160., tc); // west
    vis_wall(commands, 525., 460., 10., 160., tc); // east
    vis_wall(commands, 405., 380., 60., 10., tc); // south-left  (375..435)
    vis_wall(commands, 495., 380., 60., 10., tc); // south-right (465..525)
    // doorway gap is at x=450 ± 15 (30px wide)
    // Concrete platform inset
    rect(
        commands,
        450.,
        460.,
        130.,
        130.,
        Color::srgb(0.62, 0.62, 0.64),
        1.4,
    );
    // Yellow safety stripe along the south edge of the platform
    rect(
        commands,
        450.,
        400.,
        130.,
        4.,
        Color::srgb(0.95, 0.78, 0.18),
        1.55,
    );
    // Stylised bus parked along the back wall
    rect(
        commands,
        450.,
        510.,
        100.,
        26.,
        Color::srgb(0.85, 0.78, 0.30),
        1.5,
    );
    rect(
        commands,
        420.,
        514.,
        14.,
        12.,
        Color::srgb(0.32, 0.42, 0.58),
        1.6,
    );
    rect(
        commands,
        450.,
        514.,
        14.,
        12.,
        Color::srgb(0.32, 0.42, 0.58),
        1.6,
    );
    rect(
        commands,
        480.,
        514.,
        14.,
        12.,
        Color::srgb(0.32, 0.42, 0.58),
        1.6,
    );
    // Departures sign
    rect(
        commands,
        450.,
        478.,
        70.,
        10.,
        Color::srgb(0.10, 0.12, 0.18),
        1.6,
    );
    // TRANSIT interactables
    obj(
        commands,
        420.,
        440.,
        22.,
        18.,
        Color::srgb(0.85, 0.55, 0.18),
        ActionKind::BuyTransport,
        "[E] Buy Transit Pass",
    );
    obj(
        commands,
        480.,
        440.,
        22.,
        12.,
        Color::srgb(0.45, 0.32, 0.22),
        ActionKind::Relax,
        "[E] Wait on Bench",
    );
    obj(
        commands,
        450.,
        420.,
        20.,
        14.,
        Color::srgb(0.72, 0.32, 0.20),
        ActionKind::UseItem(ItemKind::Coffee),
        "[E] Vending Coffee",
    );

    // -- Building classification markers ---------------------------------------
    commands.spawn(Building {
        name: "HOME",
        kind: BuildingKind::Individual,
    });
    commands.spawn(Building {
        name: "GYM",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "LIBRARY",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "PARK",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "OFFICE",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "BANK",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "HOSPITAL",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "MARKET",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "RESTAURANT",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "ADOPTION",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "GARAGE",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "SCHOOL",
        kind: BuildingKind::Collective,
    });
    commands.spawn(Building {
        name: "TRANSIT",
        kind: BuildingKind::Collective,
    });

    // -- Building walls with doorways -------------------------------------------
    // Wall thickness = 10. Door gap = 50 (player is 18px wide).
    // N-row buildings: door on SOUTH face (y=80).
    // S-row buildings: door on NORTH face (y=-80).

    // HOME (-425, 180, 180x220) - south door at x=-425
    let c = Color::srgb(0.50, 0.36, 0.22);
    let f = Color::srgb(0.62, 0.44, 0.28);
    vis_wall(commands, -425., 290., 180., 10., c); // north
    vis_wall(commands, -515., 180., 10., 220., c); // west
    vis_wall(commands, -335., 180., 10., 220., c); // east
    vis_wall(commands, -482.5, 70., 65., 10., c); // south-left
    vis_wall(commands, -367.5, 70., 65., 10., c); // south-right
    rect(commands, -450., 72., 8., 10., f, 1.5);
    rect(commands, -400., 72., 8., 10., f, 1.5);

    // WELLNESS (-255, 180, 150x200) - south door at x=-255
    let c = Color::srgb(0.22, 0.46, 0.40);
    vis_wall(commands, -255., 280., 150., 10., c); // north
    vis_wall(commands, -330., 180., 10., 200., c); // west
    vis_wall(commands, -180., 180., 10., 200., c); // east
    vis_wall(commands, -305., 80., 50., 10., c); // south-left
    vis_wall(commands, -205., 80., 50., 10., c); // south-right
    rect(commands, -280., 82., 8., 10., f, 1.5);
    rect(commands, -230., 82., 8., 10., f, 1.5);

    // LIBRARY (-85, 180, 180x220) - south door at x=-85
    let c = Color::srgb(0.18, 0.28, 0.44);
    let f = Color::srgb(0.26, 0.38, 0.56);
    vis_wall(commands, -85., 290., 180., 10., c); // north
    vis_wall(commands, -175., 180., 10., 220., c); // west
    vis_wall(commands, 5., 180., 10., 220., c); // east
    vis_wall(commands, -142.5, 70., 65., 10., c); // south-left
    vis_wall(commands, -27.5, 70., 65., 10., c); // south-right
    rect(commands, -110., 72., 8., 10., f, 1.5);
    rect(commands, -60., 72., 8., 10., f, 1.5);

    // PARK (85, 180, 150x160) - open, no walls

    // OFFICE (425, 180, 180x220) - south door at x=425
    let c = Color::srgb(0.25, 0.32, 0.45);
    let f = Color::srgb(0.35, 0.44, 0.60);
    vis_wall(commands, 425., 290., 180., 10., c); // north
    vis_wall(commands, 335., 180., 10., 220., c); // west
    vis_wall(commands, 515., 180., 10., 220., c); // east
    vis_wall(commands, 367.5, 70., 65., 10., c); // south-left
    vis_wall(commands, 482.5, 70., 65., 10., c); // south-right
    rect(commands, 400., 72., 8., 10., f, 1.5);
    rect(commands, 450., 72., 8., 10., f, 1.5);

    // BANK (-425, -180, 150x200) - north door at x=-425
    let c = Color::srgb(0.40, 0.34, 0.20);
    let f = Color::srgb(0.55, 0.48, 0.30);
    vis_wall(commands, -425., -280., 150., 10., c); // south
    vis_wall(commands, -500., -180., 10., 200., c); // west
    vis_wall(commands, -350., -180., 10., 200., c); // east
    vis_wall(commands, -475., -80., 50., 10., c); // north-left
    vis_wall(commands, -375., -80., 50., 10., c); // north-right
    rect(commands, -450., -82., 8., 10., f, 1.5);
    rect(commands, -400., -82., 8., 10., f, 1.5);

    // CLINIC (-255, -180, 150x200) - north door at x=-255
    let c = Color::srgb(0.60, 0.68, 0.65);
    vis_wall(commands, -255., -280., 150., 10., c); // south
    vis_wall(commands, -330., -180., 10., 200., c); // west
    vis_wall(commands, -180., -180., 10., 200., c); // east
    vis_wall(commands, -305., -80., 50., 10., c); // north-left
    vis_wall(commands, -205., -80., 50., 10., c); // north-right
    rect(commands, -280., -82., 8., 10., f, 1.5);
    rect(commands, -230., -82., 8., 10., f, 1.5);

    // STORE (-85, -180, 150x200) - north door at x=-85
    let c = Color::srgb(0.20, 0.36, 0.42);
    let f = Color::srgb(0.28, 0.48, 0.56);
    vis_wall(commands, -85., -280., 150., 10., c); // south
    vis_wall(commands, -160., -180., 10., 200., c); // west
    vis_wall(commands, -10., -180., 10., 200., c); // east
    vis_wall(commands, -135., -80., 50., 10., c); // north-left
    vis_wall(commands, -35., -80., 50., 10., c); // north-right
    rect(commands, -110., -82., 8., 10., f, 1.5);
    rect(commands, -60., -82., 8., 10., f, 1.5);

    // CAFÉ (85, -180, 150x200) - north door at x=85
    let c = Color::srgb(0.60, 0.48, 0.28);
    let f = Color::srgb(0.72, 0.58, 0.38);
    vis_wall(commands, 85., -280., 150., 10., c); // south
    vis_wall(commands, 10., -180., 10., 200., c); // west
    vis_wall(commands, 160., -180., 10., 200., c); // east
    vis_wall(commands, 35., -80., 50., 10., c); // north-left
    vis_wall(commands, 135., -80., 50., 10., c); // north-right
    rect(commands, 60., -82., 8., 10., f, 1.5);
    rect(commands, 110., -82., 8., 10., f, 1.5);

    // ADOPTION (255, -180, 150x200) - north door at x=255
    let c = Color::srgb(0.44, 0.34, 0.58);
    let f = Color::srgb(0.56, 0.44, 0.70);
    vis_wall(commands, 255., -280., 150., 10., c); // south
    vis_wall(commands, 180., -180., 10., 200., c); // west
    vis_wall(commands, 330., -180., 10., 200., c); // east
    vis_wall(commands, 205., -80., 50., 10., c); // north-left
    vis_wall(commands, 305., -80., 50., 10., c); // north-right
    rect(commands, 230., -82., 8., 10., f, 1.5);
    rect(commands, 280., -82., 8., 10., f, 1.5);

    // GARAGE (425, -180, 150x200) - north door at x=425, 70px wide for car
    let c = Color::srgb(0.26, 0.24, 0.32);
    vis_wall(commands, 425., -280., 150., 10., c); // south
    vis_wall(commands, 350., -180., 10., 200., c); // west
    vis_wall(commands, 500., -180., 10., 200., c); // east
    vis_wall(commands, 370., -80., 40., 10., c); // north-left  (350 to 390)
    vis_wall(commands, 480., -80., 40., 10., c); // north-right (460 to 500)
}

fn rect(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) {
    cmd.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(w * S, h * S)),
            ..default()
        },
        Transform::from_xyz(x * S, y * S, z),
    ));
}

fn lamp_post(cmd: &mut Commands, x: f32, y: f32) {
    // Pole
    rect(cmd, x, y - 10., 4., 36., Color::srgb(0.32, 0.30, 0.28), 1.1);
    // Head cap
    rect(cmd, x, y + 8., 14., 4., Color::srgb(0.28, 0.26, 0.24), 1.12);
    // Warm glow dot
    rect(
        cmd,
        x,
        y + 8.,
        8.,
        8.,
        Color::srgba(1.0, 0.90, 0.55, 0.80),
        1.15,
    );
    // Collision (covers the pole)
    cmd.spawn((
        Transform::from_xyz(x * S, (y - 10.) * S, 0.),
        Collider(Vec2::new(4. * S, 18. * S)),
    ));
}

/// Spawns an invisible AABB collision wall (no visual).
fn wall(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32) {
    cmd.spawn((
        Transform::from_xyz(x * S, y * S, 0.),
        Collider(Vec2::new(w * 0.5 * S, h * 0.5 * S)),
    ));
}

/// Spawns a visible wall sprite with AABB collision.
fn vis_wall(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color) {
    cmd.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(w * S, h * S)),
            ..default()
        },
        Transform::from_xyz(x * S, y * S, 1.45),
        Collider(Vec2::new(w * 0.5 * S, h * 0.5 * S)),
    ));
}

fn zone_label(cmd: &mut Commands, x: f32, y: f32, height: f32, label: &str) {
    // The floor colour is provided by the tilemap; this spawns only the text label.
    cmd.spawn((
        Text2d::new(label),
        TextFont {
            font_size: 14. * S,
            ..default()
        },
        TextColor(Color::srgba(1., 1., 1., 0.50)),
        Transform::from_xyz(x * S, (y + height / 2. - 16.) * S, 5.),
    ));
}

#[allow(clippy::too_many_arguments)]
fn obj(
    cmd: &mut Commands,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    action: ActionKind,
    prompt: &str,
) {
    cmd.spawn((
        Sprite {
            color: Color::srgba(0., 0., 0., 0.45),
            custom_size: Some(Vec2::new((w + 4.) * S, (h + 4.) * S)),
            ..default()
        },
        Transform::from_xyz((x + 2.) * S, (y - 2.) * S, 1.95),
    ));
    cmd.spawn((
        Sprite {
            color,
            custom_size: Some(Vec2::new(w * S, h * S)),
            ..default()
        },
        Transform::from_xyz(x * S, y * S, 2.),
        Interactable {
            action,
            prompt: prompt.to_string(),
        },
        ObjectSize(Vec2::new(w * S, h * S)),
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_npc(
    cmd: &mut Commands,
    name: &str,
    npc_id: usize,
    outfit: Color,
    pants: Color,
    skin: Color,
    hair: Color,
    zone_center: Vec2,
    zone_half: f32,
    seed: u64,
    personality: NpcPersonality,
    home_zone: Vec2,
    work_zone: Vec2,
) {
    let id = cmd
        .spawn((
            Transform::from_xyz(zone_center.x, zone_center.y, 9.),
            Visibility::default(),
            Npc {
                name: name.to_string(),
                wander_timer: 0.,
                target: zone_center,
                zone_center,
                zone_half,
                rng: seed,
                velocity: Vec2::ZERO,
                personality,
                home_zone,
                work_zone,
            },
            Interactable {
                action: ActionKind::Chat,
                prompt: format!("[E] Chat with {}", name),
            },
            ObjectSize(Vec2::splat(18.)),
            NpcId(npc_id),
        ))
        .with_children(|p| {
            spawn_human(p, outfit, pants, skin, hair);
        })
        .id();
    // Label floats above hair (hair tip at local y≈+15*S)
    cmd.spawn((
        Text2d::new(name),
        TextFont {
            font_size: 11.,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(zone_center.x, zone_center.y + 26. * S, 11.),
        NpcLabel(id),
    ));
}

pub fn spawn_typing_overlay(cmd: &mut Commands) {
    cmd.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(24.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.)),
        ZIndex(200),
        Visibility::Hidden,
        TypingOverlay,
        TypingOverlayFade::default(),
    ))
    .with_children(|p| {
        // Action label (e.g., "WORK")
        p.spawn((
            Text::new(""),
            TextFont {
                font_size: 22.,
                ..default()
            },
            TextColor(Color::srgb(1., 0.85, 0.25)),
            TypingLabel,
        ));
        // Word row: typed | current char in highlight box | remaining
        // Note: do NOT spawn an explicit Transform here. `Node` injects one via
        // required components, and supplying our own alongside `Node` triggers
        // a wasm "unreachable" trap at startup on Bevy 0.15. The
        // `update_typing_word_row_scale` system writes the scale every frame
        // from `TypingWordRowScale` (which defaults to START_SCALE), so the
        // first visible frame already shows the entrance scale.
        p.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            },
            TypingWordRow,
            TypingWordRowScale::default(),
        ))
        .with_children(|row| {
            // Typed chars (green)
            row.spawn((
                Text::new(""),
                TextFont {
                    font_size: 72.,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 1., 0.4)),
                TypingWordTyped,
            ));
            // Current char highlight box
            row.spawn((
                Node {
                    padding: UiRect::axes(Val::Px(3.), Val::Px(1.)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.95, 0.95, 0.8)),
                TypingWordCurrentBox,
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 72.,
                        ..default()
                    },
                    TextColor(Color::srgb(0.05, 0.05, 0.05)),
                    TypingWordCurrent,
                ));
            });
            // Remaining chars (gray)
            row.spawn((
                Text::new(""),
                TextFont {
                    font_size: 72.,
                    ..default()
                },
                TextColor(Color::srgb(0.45, 0.45, 0.45)),
                TypingWordRemaining,
            ));
        });
        // Instruction text
        p.spawn((
            Text::new(""),
            TextFont {
                font_size: 15.,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            TypingInstruction,
        ));
        // Retries / cancel hint
        p.spawn((
            Text::new(""),
            TextFont {
                font_size: 13.,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.5, 0.3)),
            TypingRetries,
        ));
    });
}

/// Spawns the skill tree panel. Hidden by default; toggled by Tab key.
/// Displays a title and four labelled bars for cooking, career, fitness, social.
pub fn spawn_skill_panel(cmd: &mut Commands) {
    cmd.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.),
            bottom: Val::Px(12.),
            padding: UiRect::all(Val::Px(14.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.),
            min_width: Val::Px(220.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.82)),
        BorderRadius::all(Val::Px(8.)),
        ZIndex(100),
        Visibility::Hidden,
        SkillPanel,
    ))
    .with_children(|p| {
        // Title
        p.spawn((
            Text::new("Skills  [Tab]"),
            TextFont {
                font_size: 14.,
                ..default()
            },
            TextColor(Color::srgb(1., 0.85, 0.25)),
        ));
        skill_row(p, "Cooking  ", SkillCookingBar);
        skill_row(p, "Career   ", SkillCareerBar);
        skill_row(p, "Fitness  ", SkillFitnessBar);
        skill_row(p, "Social   ", SkillSocialBar);
    });
}

fn skill_row<M: Component>(parent: &mut ChildBuilder, label: &'static str, marker: M) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 13.,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
            row.spawn((
                Text::new("·····"),
                TextFont {
                    font_size: 13.,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
                marker,
            ));
        });
}

/// Spawns the full-screen tutorial overlay. Hidden by default.
/// Made visible by `update_tutorial` when `TutorialState::step > 0`.
pub fn spawn_tutorial_overlay(cmd: &mut Commands) {
    cmd.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            position_type: PositionType::Absolute,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.84)),
        ZIndex(300),
        Visibility::Hidden,
        TutorialOverlay,
    ))
    .with_children(|p| {
        // Card container
        p.spawn((
            Node {
                padding: UiRect::all(Val::Px(32.)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(18.),
                max_width: Val::Px(520.),
                min_width: Val::Px(380.),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.10, 0.95)),
            BorderRadius::all(Val::Px(12.)),
        ))
        .with_children(|card| {
            // Step counter (e.g. "1 / 6")
            card.spawn((
                Text::new(""),
                TextFont {
                    font_size: 12.,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                TutorialHintText,
            ));
            // Body text (title + content rendered as one block)
            card.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.,
                    ..default()
                },
                TextColor(Color::WHITE),
                TutorialBodyText,
            ));
            // Dismiss hint
            card.spawn((
                Text::new("[Space] / [Enter] Next   [Esc] Skip"),
                TextFont {
                    font_size: 12.,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.55, 0.55)),
            ));
        });
    });
}

pub fn spawn_hud(cmd: &mut Commands) {
    cmd.spawn(Node {
        width: Val::Percent(100.),
        height: Val::Percent(100.),
        position_type: PositionType::Absolute,
        ..default()
    })
    .with_children(|root| {
        spawn_hud_left_panel(root);
        spawn_hud_right_panel(root);
        spawn_hud_notification_area(root);
        spawn_hud_prompt_overlay(root);
    });
}

fn spawn_hud_left_panel(root: &mut ChildBuilder) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.),
            top: Val::Px(12.),
            padding: UiRect::all(Val::Px(10.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.72)),
        BorderRadius::all(Val::Px(8.)),
    ))
    .with_children(|p| {
        htxt(
            p,
            "Day 1  08:00 AM  (Mon)",
            15.,
            Color::srgb(1., 0.9, 0.4),
            HudLabel::Time,
        );
        htxt(
            p,
            "$100 cash  savings $0  | 2 meals",
            12.,
            Color::srgb(0.4, 1., 0.5),
            HudLabel::Money,
        );
        htxt(p, "", 11., Color::srgb(0.8, 0.4, 0.4), HudLabel::Rent);
        htxt(
            p,
            "Mood: Happy | Junior | Apartment",
            12.,
            Color::srgb(0.8, 0.9, 1.0),
            HudLabel::Mood,
        );
        stat_bar(p, "Energy    ", Color::srgb(1., 0.78, 0.2), HudBar::Energy);
        stat_bar(p, "Satiety   ", Color::srgb(1., 0.44, 0.2), HudBar::Hunger);
        stat_bar(
            p,
            "Happiness ",
            Color::srgb(0.4, 0.8, 1.),
            HudBar::Happiness,
        );
        stat_bar(p, "Health    ", Color::srgb(0.3, 0.9, 0.4), HudBar::Health);
        stat_bar(p, "Stress    ", Color::srgb(0.9, 0.3, 0.2), HudBar::Stress);
        htxt(p, "", 12., Color::srgb(1., 0.3, 0.3), HudLabel::Warning);
        htxt(
            p,
            "Streak: 0 days  Loan: $0",
            11.,
            Color::srgb(0.9, 0.7, 0.4),
            HudLabel::Streak,
        );
        p.spawn((
            Text::new("-- Skills --"),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        ));
        htxt(
            p,
            "Cook 0.0   Career 0.0\nFit  0.0   Social 0.0",
            11.,
            Color::srgb(0.75, 0.85, 1.0),
            HudLabel::Skills,
        );
        p.spawn((
            Text::new("-- Friends --"),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        ));
        htxt(
            p,
            "Alex 0/5  Sam 0/5  Mia 0/5",
            11.,
            Color::srgb(1.0, 0.65, 0.75),
            HudLabel::Friendship,
        );
        p.spawn((
            Text::new("-- Inventory --"),
            TextFont {
                font_size: 11.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.5, 0.5)),
        ));
        htxt(
            p,
            "Coffee x0  Vitamins x0  Books x0",
            11.,
            Color::srgb(0.9, 0.8, 0.6),
            HudLabel::Inventory,
        );
    });
}

fn spawn_hud_right_panel(root: &mut ChildBuilder) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(12.),
            top: Val::Px(12.),
            padding: UiRect::all(Val::Px(10.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
            max_width: Val::Px(255.),
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.72)),
        BorderRadius::all(Val::Px(8.)),
    ))
    .with_children(|p| {
        p.spawn((
            Text::new("-- Daily Goal --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.75, 0.2)),
        ));
        htxt(p, "...", 12., Color::WHITE, HudLabel::Goal);
        p.spawn((
            Text::new("-- Story --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.72, 0.55)),
        ));
        htxt(
            p,
            "Your story is just beginning.",
            11.,
            Color::srgb(1.0, 0.88, 0.75),
            HudLabel::Story,
        );
        p.spawn((
            Text::new("-- Weather --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.8, 1.0)),
        ));
        htxt(
            p,
            "Sunny — outdoor bonus",
            11.,
            Color::srgb(1.0, 0.95, 0.6),
            HudLabel::Weather,
        );
        p.spawn((
            Text::new("-- Hobbies --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.6, 0.9)),
        ));
        htxt(
            p,
            "Paint 0.0  Game 0.0  Music 0.0",
            11.,
            Color::srgb(0.85, 0.75, 1.0),
            HudLabel::Hobbies,
        );
        p.spawn((
            Text::new("-- Conditions --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.4, 0.3)),
        ));
        htxt(
            p,
            "Healthy",
            11.,
            Color::srgb(0.4, 0.9, 0.5),
            HudLabel::Conditions,
        );
        p.spawn((
            Text::new("-- Reputation --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.7, 0.4)),
        ));
        htxt(
            p,
            "Rep: 0/100",
            11.,
            Color::srgb(1.0, 0.85, 0.5),
            HudLabel::Reputation,
        );
        p.spawn((
            Text::new("-- Season --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.5, 0.9, 0.7)),
        ));
        htxt(
            p,
            "Spring — social bonus",
            11.,
            Color::srgb(0.7, 1.0, 0.8),
            HudLabel::Season,
        );
        p.spawn((
            Text::new("-- Pet --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.75, 0.5)),
        ));
        htxt(
            p,
            "No pet (adopt at Pet Bowl)",
            11.,
            Color::srgb(0.9, 0.8, 0.6),
            HudLabel::Pet,
        );
        p.spawn((
            Text::new("-- Transport --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.85)),
        ));
        htxt(
            p,
            "On foot (buy at Garage)",
            11.,
            Color::srgb(0.8, 0.8, 0.95),
            HudLabel::Transport,
        );
        p.spawn((
            Text::new("-- Quests --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.8, 0.5)),
        ));
        htxt(
            p,
            "No quests - chat with NPCs [Q]",
            11.,
            Color::srgb(0.95, 0.9, 0.65),
            HudLabel::Quest,
        );
        p.spawn((
            Text::new("-- Life Rating --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.9, 0.6)),
        ));
        htxt(
            p,
            "B — Comfortable",
            13.,
            Color::srgb(0.8, 1.0, 0.7),
            HudLabel::Rating,
        );
        p.spawn((
            Text::new("-- Housing --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.65, 0.5)),
        ));
        htxt(
            p,
            "Apartment | $20/day | [E] Bank to upgrade",
            11.,
            Color::srgb(0.82, 0.76, 0.65),
            HudLabel::Housing,
        );
        p.spawn((
            Text::new("-- Milestones --"),
            TextFont {
                font_size: 12.,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.85, 0.3)),
        ));
        htxt(
            p,
            "None yet  (0/21)",
            11.,
            Color::srgb(0.95, 0.90, 0.6),
            HudLabel::Milestones,
        );
    });
}

fn spawn_hud_notification_area(root: &mut ChildBuilder) {
    root.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(0.),
            right: Val::Px(0.),
            justify_content: JustifyContent::Center,
            ..default()
        },
        NotifContainer,
    ))
    .with_children(|top| {
        top.spawn(Node {
            padding: UiRect::axes(Val::Px(16.), Val::Px(8.)),
            ..default()
        })
        .with_children(|inner| {
            inner.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.,
                    ..default()
                },
                TextColor(Color::srgb(1., 0.88, 0.3)),
                HudLabel::Notification,
            ));
        });
    });
}

fn spawn_hud_prompt_overlay(root: &mut ChildBuilder) {
    root.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(18.),
        left: Val::Px(0.),
        right: Val::Px(0.),
        justify_content: JustifyContent::Center,
        ..default()
    })
    .with_children(|b| {
        b.spawn((
            Node {
                padding: UiRect::axes(Val::Px(14.), Val::Px(7.)),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.72)),
        ))
        .with_children(|inner| {
            inner.spawn((
                Text::new(""),
                TextFont {
                    font_size: 15.,
                    ..default()
                },
                TextColor(Color::WHITE),
                HudLabel::Prompt,
            ));
        });
    });
}

fn htxt(parent: &mut ChildBuilder, text: &str, size: f32, color: Color, label: HudLabel) {
    parent.spawn((
        Text::new(text),
        TextFont {
            font_size: size,
            ..default()
        },
        TextColor(color),
        label,
    ));
}

fn stat_bar(parent: &mut ChildBuilder, label: &str, color: Color, bar: HudBar) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(5.),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: 12.,
                    ..default()
                },
                TextColor(Color::srgb(0.78, 0.78, 0.78)),
            ));
            row.spawn((
                Node {
                    width: Val::Px(90.),
                    height: Val::Px(9.),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                BorderRadius::all(Val::Px(4.)),
            ))
            .with_children(|track| {
                track.spawn((
                    Node {
                        width: Val::Percent(80.),
                        height: Val::Percent(100.),
                        ..default()
                    },
                    BackgroundColor(color),
                    bar,
                    BarSmooth {
                        displayed: 80.,
                        target: 80.,
                    },
                ));
            });
        });
}
