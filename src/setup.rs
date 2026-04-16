use bevy::prelude::*;
use crate::components::{
    ActionKind, BodyPart, Collider, DayNightOverlay, HobbyKind, HudBar, HudLabel,
    InteractHighlight, Interactable, ItemKind, MainCamera, Npc, NpcId, NpcLabel,
    ObjectSize, Player, PlayerIndicator,
};

/// Builds a composite human figure as child entities of the calling spawn.
/// The root entity should have Transform + Visibility but no Sprite.
///
/// Body layout (local coords, root at 0,0):
///   shadow y=-14, feet y=-10, legs y=-5, torso y=1, head y=9, hair y=13
fn spawn_human(p: &mut ChildBuilder, outfit: Color, pants: Color, skin: Color, hair: Color) {
    // Ground shadow
    p.spawn((Sprite { color: Color::srgba(0., 0., 0., 0.32), custom_size: Some(Vec2::new(20., 7.)), ..default() },
              Transform::from_xyz(2., -14., -1.)));
    // Left shoe
    p.spawn((Sprite { color: Color::srgb(0.14, 0.10, 0.07), custom_size: Some(Vec2::new(4., 4.)), ..default() },
              Transform::from_xyz(-4., -10., 0.5), BodyPart::LeftFoot));
    // Right shoe
    p.spawn((Sprite { color: Color::srgb(0.14, 0.10, 0.07), custom_size: Some(Vec2::new(4., 4.)), ..default() },
              Transform::from_xyz(4., -10., 0.5), BodyPart::RightFoot));
    // Left leg
    p.spawn((Sprite { color: pants, custom_size: Some(Vec2::new(4., 7.)), ..default() },
              Transform::from_xyz(-4., -5., 1.), BodyPart::LeftLeg));
    // Right leg
    p.spawn((Sprite { color: pants, custom_size: Some(Vec2::new(4., 7.)), ..default() },
              Transform::from_xyz(4., -5., 1.), BodyPart::RightLeg));
    // Torso
    p.spawn((Sprite { color: outfit, custom_size: Some(Vec2::new(12., 10.)), ..default() },
              Transform::from_xyz(0., 1., 1.5), BodyPart::Body));
    // Head
    p.spawn((Sprite { color: skin, custom_size: Some(Vec2::new(9., 9.)), ..default() },
              Transform::from_xyz(0., 9., 2.), BodyPart::Head));
    // Hair
    p.spawn((Sprite { color: hair, custom_size: Some(Vec2::new(10., 4.)), ..default() },
              Transform::from_xyz(0., 13., 2.5), BodyPart::Hair));
    // Left eye
    p.spawn((Sprite { color: Color::srgb(0.08, 0.05, 0.04), custom_size: Some(Vec2::new(2., 2.)), ..default() },
              Transform::from_xyz(-2., 9., 3.)));
    // Right eye
    p.spawn((Sprite { color: Color::srgb(0.08, 0.05, 0.04), custom_size: Some(Vec2::new(2., 2.)), ..default() },
              Transform::from_xyz(2., 9., 3.)));
}

pub fn setup(mut commands: Commands) {
    commands.spawn((Camera2d, MainCamera));

    // ── Ground ────────────────────────────────────────────────────────────────
    rect(&mut commands, 0., 0., 3000., 3000., Color::srgb(0.28, 0.26, 0.23), 0.);

    // ── Ground scatter patches (texture variation) ─────────────────────────
    for (px, py, pw, ph) in [
        (-300., 220., 55., 36.), (-180., -260., 48., 30.), (310., -185., 42., 28.),
        (-360., 310., 36., 24.), (240., 310., 52., 34.), (-240., -360., 38., 26.),
        ( 360., 370., 44., 30.), (-140., 360., 40., 26.), (145., -305., 50., 32.),
        (-410., -190., 34., 22.), (415., 195., 38., 24.), (355., -330., 30., 20.),
        (-320., -130., 46., 30.), (280., -320., 44., 28.), (-260., 170., 38., 24.),
    ] {
        rect(&mut commands, px, py, pw, ph, Color::srgb(0.26, 0.24, 0.21), 0.15);
    }

    // ── Sidewalks flanking roads ──────────────────────────────────────────
    let sw = Color::srgb(0.42, 0.40, 0.36);
    rect(&mut commands,  0.,  72., 3000., 14., sw, 0.62);
    rect(&mut commands,  0., -72., 3000., 14., sw, 0.62);
    rect(&mut commands,  72., 0., 14., 3000., sw, 0.62);
    rect(&mut commands, -72., 0., 14., 3000., sw, 0.62);

    // ── Crosswalk stripes at intersection ─────────────────────────────────
    let cw = Color::srgba(0.95, 0.95, 0.88, 0.32);
    for i in 0..6i32 {
        // Crossing vertical road (horizontal stripes, x from -55 to 55)
        let cx = -46. + i as f32 * 18.;
        rect(&mut commands, cx, 0., 7., 110., cw, 0.78);
        // Crossing horizontal road (vertical stripes, y from -55 to 55)
        let cy = -46. + i as f32 * 18.;
        rect(&mut commands, 0., cy, 110., 7., cw, 0.78);
    }

    // ── Roads ─────────────────────────────────────────────────────────────────
    rect(&mut commands, 0., 0., 3000., 110.,  Color::srgb(0.36, 0.34, 0.30), 0.5);
    rect(&mut commands, 0., 0., 110.,  3000., Color::srgb(0.36, 0.34, 0.30), 0.5);
    // Crossroad highlight
    rect(&mut commands, 0., 0., 110., 110., Color::srgb(0.34, 0.32, 0.29), 0.55);
    // Road edge lines
    rect(&mut commands, 0.,  55., 3000., 2., Color::srgba(1., 1., 0.8, 0.10), 0.6);
    rect(&mut commands, 0., -55., 3000., 2., Color::srgba(1., 1., 0.8, 0.10), 0.6);
    rect(&mut commands,  55., 0., 2., 3000., Color::srgba(1., 1., 0.8, 0.10), 0.6);
    rect(&mut commands, -55., 0., 2., 3000., Color::srgba(1., 1., 0.8, 0.10), 0.6);

    // Dashed center lines (horizontal road y=0)
    for i in -17i32..=17 {
        let x = i as f32 * 40.;
        if x.abs() < 58. { continue; } // skip crossroads gap
        rect(&mut commands, x, 0., 18., 3., Color::srgba(1., 1., 0.75, 0.20), 0.7);
    }
    // Dashed center lines (vertical road x=0)
    for i in -17i32..=17 {
        let y = i as f32 * 40.;
        if y.abs() < 58. { continue; }
        rect(&mut commands, 0., y, 3., 18., Color::srgba(1., 1., 0.75, 0.20), 0.7);
    }

    // ── Lamp posts along roads ─────────────────────────────────────────────
    for &(lx, ly) in &[
        (-350., 72.), (-200., 72.), (200., 72.), (350., 72.),
        (-350.,-72.), (-200.,-72.), (200.,-72.), (350.,-72.),
        ( 72., 250.), ( 72., 350.), (-72., 250.), (-72., 350.),
        ( 72.,-250.), ( 72.,-350.), (-72.,-250.), (-72.,-350.),
    ] {
        lamp_post(&mut commands, lx, ly);
    }

    // ── Zones ─────────────────────────────────────────────────────────────────
    zone(&mut commands, -500.,  0.,   360., 360., Color::srgb(0.72, 0.58, 0.42), "HOME");
    zone(&mut commands,  500.,  0.,   360., 360., Color::srgb(0.42, 0.52, 0.68), "OFFICE");
    zone(&mut commands,    0.,  450., 360., 360., Color::srgb(0.28, 0.58, 0.28), "PARK");
    zone(&mut commands,    0., -450., 360., 360., Color::srgb(0.32, 0.52, 0.58), "STORE");
    zone(&mut commands, -500., -450., 240., 240., Color::srgb(0.55, 0.48, 0.32), "BANK");
    zone(&mut commands,  500.,  450., 240., 240., Color::srgb(0.30, 0.42, 0.58), "LIBRARY");
    zone(&mut commands,  500., -450., 240., 240., Color::srgb(0.40, 0.38, 0.45), "GARAGE");

    // ── Zone building details ──────────────────────────────────────────────
    // HOME (-500, 0, 360x360) — warm residential
    let wc = Color::srgb(0.82, 0.92, 0.98); // window glass
    rect(&mut commands, -500., 170., 360., 16., Color::srgb(0.50, 0.36, 0.22), 1.15); // roof ridge
    for wx in &[-610., -500., -390.] {
        rect(&mut commands, *wx, 110., 28., 20., wc, 1.2);
        rect(&mut commands, *wx, 110., 32., 24., Color::srgba(0., 0., 0., 0.18), 1.18); // frame shadow
    }
    rect(&mut commands, -500., -155., 20., 36., Color::srgb(0.28, 0.16, 0.06), 1.2); // door
    rect(&mut commands, -500., -155., 24., 40., Color::srgba(0., 0., 0., 0.28), 1.18); // door shadow

    // OFFICE (500, 0, 360x360) — sleek corporate glass facade
    rect(&mut commands, 500., 170., 360., 14., Color::srgb(0.25, 0.32, 0.45), 1.15); // roofline
    for wx in &[380., 450., 520., 590.] {
        for wy in &[100., 40., -20., -80.] {
            rect(&mut commands, *wx, *wy, 22., 16., wc, 1.2);
            rect(&mut commands, *wx, *wy, 26., 20., Color::srgba(0., 0., 0., 0.15), 1.18);
        }
    }
    // Glass entrance
    rect(&mut commands, 500., -150., 42., 50., Color::srgb(0.60, 0.82, 0.95), 1.2);
    rect(&mut commands, 500., -150., 46., 54., Color::srgba(0., 0., 0., 0.22), 1.18);

    // PARK (0, 450, 360x360) — grassy with path
    // Central dirt path
    rect(&mut commands, 0., 390., 24., 120., Color::srgb(0.44, 0.36, 0.26), 1.12);
    // Flower beds (small colored clusters)
    for (fx, fy, fc) in [
        (-120., 490., Color::srgb(0.95, 0.40, 0.55)),
        (-80.,  480., Color::srgb(1.00, 0.82, 0.20)),
        ( 120., 490., Color::srgb(0.55, 0.75, 0.95)),
        ( 80.,  480., Color::srgb(0.90, 0.50, 0.80)),
        (-140., 420., Color::srgb(1.00, 0.70, 0.25)),
        ( 140., 420., Color::srgb(0.55, 0.90, 0.55)),
    ] {
        rect(&mut commands, fx,     fy,     14., 10., fc,                             3.1);
        rect(&mut commands, fx + 8., fy - 5., 10., 8., Color::srgb(0.18, 0.52, 0.18), 3.05);
    }
    // Bench
    rect(&mut commands, -50., 415., 28., 6., Color::srgb(0.50, 0.32, 0.14), 1.2);
    rect(&mut commands, -50., 420., 28., 3., Color::srgb(0.40, 0.25, 0.10), 1.21);

    // STORE (0, -450, 360x360) — shop facade
    rect(&mut commands, 0., -270., 360., 14., Color::srgb(0.22, 0.38, 0.45), 1.15); // top fascia
    // Display windows
    rect(&mut commands, -90., -310., 80., 50., wc, 1.2);
    rect(&mut commands,  90., -310., 80., 50., wc, 1.2);
    rect(&mut commands, -90., -310., 84., 54., Color::srgba(0., 0., 0., 0.15), 1.18);
    rect(&mut commands,  90., -310., 84., 54., Color::srgba(0., 0., 0., 0.15), 1.18);
    // Awning strip
    rect(&mut commands, 0., -278., 280., 10., Color::srgb(0.85, 0.22, 0.22), 1.25);

    // BANK (-500, -450, 240x240) — dignified columns
    rect(&mut commands, -500., -330., 240., 12., Color::srgb(0.62, 0.52, 0.30), 1.15); // cornice
    for cx in &[-585., -540., -460., -415.] {
        rect(&mut commands, *cx, -400., 14., 100., Color::srgb(0.68, 0.60, 0.40), 1.2); // column
    }
    // Steps (horizontal strips)
    for (sy, sw) in [(-545., 200.), (-552., 216.), (-559., 232.)] {
        rect(&mut commands, -500., sy, sw, 8., Color::srgb(0.60, 0.55, 0.42), 1.18);
    }

    // LIBRARY (500, 450, 240x240) — arched entrance
    rect(&mut commands, 500., 570., 240., 12., Color::srgb(0.22, 0.30, 0.48), 1.15); // roofline
    // Arched door frame (two pillars + top bar)
    rect(&mut commands, 486., 480., 12., 50., Color::srgb(0.18, 0.25, 0.40), 1.2);
    rect(&mut commands, 514., 480., 12., 50., Color::srgb(0.18, 0.25, 0.40), 1.2);
    rect(&mut commands, 500., 508., 42., 10., Color::srgb(0.18, 0.25, 0.40), 1.2);
    // Steps
    rect(&mut commands, 500., 342., 180., 8., Color::srgb(0.26, 0.35, 0.52), 1.18);
    rect(&mut commands, 500., 334., 200., 8., Color::srgb(0.22, 0.30, 0.48), 1.17);

    // GARAGE (500, -450, 240x240) — roller door lines
    rect(&mut commands, 500., -360., 200., 100., Color::srgb(0.30, 0.28, 0.34), 1.2); // door panel
    for gy in &[-320., -340., -360., -380., -400.] {
        rect(&mut commands, 500., *gy, 196., 2., Color::srgba(0., 0., 0., 0.30), 1.3); // horizontal slat lines
    }
    // Parking lot stripes in front of GARAGE
    rect(&mut commands, 500., -510., 240., 60., Color::srgb(0.35, 0.33, 0.38), 1.05);
    for i in 0..5i32 {
        let px = 390. + i as f32 * 30.;
        rect(&mut commands, px, -510., 2., 56., Color::srgba(0.85, 0.85, 0.75, 0.28), 1.1);
    }

    // ── Park pond ─────────────────────────────────────────────────────────
    // Pond (PARK center-ish, offset from bench area)
    rect(&mut commands, 60., 460., 72., 48., Color::srgb(0.15, 0.38, 0.58), 1.08); // water
    rect(&mut commands, 60., 460., 76., 52., Color::srgba(0., 0., 0., 0.28), 1.06); // pond border shadow
    rect(&mut commands, 54., 457., 34., 20., Color::srgba(0.55, 0.80, 0.95, 0.22), 1.12); // water shimmer highlight
    rect(&mut commands, 65., 470., 18., 8.,  Color::srgba(0.55, 0.80, 0.95, 0.15), 1.13); // secondary shimmer

    // ── Interactive objects ────────────────────────────────────────────────────
    // Home
    obj(&mut commands, -560.,  60., 44., 22., Color::srgb(0.45, 0.22, 0.58), ActionKind::Sleep,    "[E] Sleep");
    obj(&mut commands, -480., -44., 22., 38., Color::srgb(0.85, 0.92, 0.95), ActionKind::Eat,      "[E] Eat");
    obj(&mut commands, -440.,  72., 20., 28., Color::srgb(0.55, 0.85, 0.95), ActionKind::Shower,   "[E] Shower");
    obj(&mut commands, -530., -60., 22., 22., Color::srgb(0.70, 0.55, 0.80), ActionKind::Meditate, "[E] Meditate");
    obj(&mut commands, -545.,  22., 38., 18., Color::srgb(0.55, 0.45, 0.28), ActionKind::Freelance, "[E] Freelance Desk - work from home");
    obj(&mut commands, -612.,  32., 16., 16., Color::srgb(0.60, 0.35, 0.15), ActionKind::UseItem(ItemKind::Coffee),   "[E] Drink Coffee");
    obj(&mut commands, -612., -8.,  16., 16., Color::srgb(0.25, 0.65, 0.35), ActionKind::UseItem(ItemKind::Vitamins), "[E] Take Vitamins");
    obj(&mut commands, -612., -48., 16., 16., Color::srgb(0.25, 0.45, 0.75), ActionKind::UseItem(ItemKind::Books),    "[E] Read Book");
    obj(&mut commands, -458.,  50., 18., 22., Color::srgb(0.80, 0.65, 0.30), ActionKind::Hobby(HobbyKind::Painting),  "[E] Paint (Painting skill)");
    obj(&mut commands, -458.,  18., 18., 22., Color::srgb(0.25, 0.35, 0.65), ActionKind::Hobby(HobbyKind::Gaming),   "[E] Game (Gaming skill)");
    obj(&mut commands, -458., -14., 18., 22., Color::srgb(0.65, 0.30, 0.55), ActionKind::Hobby(HobbyKind::Music),    "[E] Play Music (Music skill)");
    obj(&mut commands, -535., -90., 16., 16., Color::srgb(0.72, 0.55, 0.35), ActionKind::FeedPet,   "[E] Pet Bowl — Feed/Adopt pet ($5 feed / $50 adopt)");
    obj(&mut commands, -440., -80., 22., 22., Color::srgb(0.85, 0.40, 0.55), ActionKind::ThrowParty,"[E] Party Corner — Throw a party! ($40)");
    // Office
    obj(&mut commands,  500.,   0., 50., 28., Color::srgb(0.50, 0.36, 0.20), ActionKind::Work, "[E] Work");
    // Store
    obj(&mut commands,    0., -450., 64., 26., Color::srgb(0.92, 0.88, 0.50), ActionKind::Shop,     "[E] Shop");
    // Park
    obj(&mut commands,    0.,  450., 44., 18., Color::srgb(0.50, 0.34, 0.18), ActionKind::Relax,    "[E] Relax");
    obj(&mut commands,   80.,  430., 22., 38., Color::srgb(0.25, 0.70, 0.35), ActionKind::Exercise, "[E] Exercise");
    // Bank
    obj(&mut commands, -500., -450., 40., 28., Color::srgb(0.85, 0.75, 0.30), ActionKind::Bank, "[E] Bank  [1-8] actions");
    // Library
    obj(&mut commands,  500.,  450., 46., 28., Color::srgb(0.55, 0.75, 0.90), ActionKind::StudyCourse, "[E] Study — $30 + 20 energy → +0.5 random skill");
    // Garage
    obj(&mut commands,  500., -450., 46., 28., Color::srgb(0.60, 0.55, 0.70), ActionKind::BuyTransport, "[E] Transport  [1] Bike $80sav  [2] Car $300sav");

    for (x, y, s) in [
        (-110., 400., 22.), (115., 415., 22.), (-65., 490., 22.), (95., 495., 22.), (-140., 488., 22.),
        (-40., 422., 16.), (150., 458., 18.), (-90., 455., 20.), (70., 418., 14.), (20., 500., 18.),
        (-160., 440., 16.), (130., 490., 16.),
    ] {
        // Drop shadow — flat ellipse beneath
        rect(&mut commands, x + 5., y - s * 0.62, s * 1.2, s * 0.38, Color::srgba(0., 0., 0., 0.28), 2.9);
        // Trunk
        rect(&mut commands, x, y - s * 0.5 + 3., s * 0.35, 7., Color::srgb(0.32, 0.20, 0.08), 2.95);
        // Outer canopy (dark green)
        rect(&mut commands, x, y, s, s, Color::srgb(0.12, 0.40, 0.12), 3.0);
        // Inner canopy highlight (mid-green, slightly up-left)
        let hs = s * 0.65;
        rect(&mut commands, x - s * 0.09, y + s * 0.07, hs, hs, Color::srgb(0.20, 0.58, 0.20), 3.05);
        // Specular dot (light green highlight)
        let ss = s * 0.30;
        rect(&mut commands, x - s * 0.20, y + s * 0.18, ss, ss, Color::srgb(0.36, 0.74, 0.28), 3.1);
    }

    spawn_npc(&mut commands, "Alex", 0,
        Color::srgb(0.80, 0.22, 0.22), Color::srgb(0.50, 0.12, 0.12), // red outfit
        Color::srgb(0.94, 0.80, 0.65), Color::srgb(0.34, 0.20, 0.08), // light skin, brown hair
        Vec2::new(0., 450.), 130., 1);
    spawn_npc(&mut commands, "Sam", 1,
        Color::srgb(0.22, 0.68, 0.32), Color::srgb(0.12, 0.42, 0.18), // green outfit
        Color::srgb(0.76, 0.58, 0.40), Color::srgb(0.10, 0.08, 0.06), // medium skin, black hair
        Vec2::new(500., 0.), 130., 2);
    spawn_npc(&mut commands, "Mia", 2,
        Color::srgb(0.58, 0.32, 0.88), Color::srgb(0.35, 0.16, 0.58), // purple outfit
        Color::srgb(0.96, 0.84, 0.70), Color::srgb(0.66, 0.20, 0.10), // light skin, auburn hair
        Vec2::new(-500., 0.), 130., 3);

    commands.spawn((
        Transform::from_xyz(0., 0., 10.),
        Visibility::default(),
        Player,
    )).with_children(|p| {
        spawn_human(p,
            Color::srgb(0.90, 0.52, 0.12), // outfit: orange
            Color::srgb(0.58, 0.28, 0.06), // pants: dark orange
            Color::srgb(0.94, 0.80, 0.65), // skin: light
            Color::srgb(0.36, 0.22, 0.09), // hair: brown
        );
        // Direction indicator dot (orbits toward movement direction)
        p.spawn((Sprite { color: Color::srgb(1., 1., 0.55), custom_size: Some(Vec2::splat(5.)), ..default() },
                  Transform::from_xyz(0., 18., 3.5), PlayerIndicator));
    });

    // Day/night ambient overlay (world-space, covers entire map, z above all sprites)
    commands.spawn((
        Sprite { color: Color::srgba(0., 0., 0.12, 0.), custom_size: Some(Vec2::splat(6000.)), ..default() },
        Transform::from_xyz(0., 0., 50.),
        DayNightOverlay,
    ));
    // Interactable proximity highlight (invisible until near something)
    commands.spawn((
        Sprite { color: Color::srgba(1., 1., 0.5, 0.), custom_size: Some(Vec2::splat(30.)), ..default() },
        Transform::from_xyz(0., 0., 1.98),
        InteractHighlight,
    ));

    // ── Collision walls ────────────────────────────────────────────────────────

    // World boundary (invisible, just outside the visible play area)
    wall(&mut commands,    0.,  740., 1500.,  20.);
    wall(&mut commands,    0., -740., 1500.,  20.);
    wall(&mut commands, -740.,    0.,  20., 1500.);
    wall(&mut commands,  740.,    0.,  20., 1500.);

    // Pond obstacle (park)
    wall(&mut commands, 60., 460., 76., 52.);

    // Tree obstacles (park canopy footprints)
    for (tx, ty, ts) in [
        (-110., 400., 22.), (115., 415., 22.), (-65., 490., 22.), (95., 495., 22.),
        (-140., 488., 22.), (-40., 422., 16.), (150., 458., 18.), (-90., 455., 20.),
        (70., 418., 14.), (20., 500., 18.), (-160., 440., 16.), (130., 490., 16.),
    ] {
        wall(&mut commands, tx, ty, ts * 0.75, ts * 0.75);
    }

    spawn_hud(&mut commands);
}

fn rect(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, z: f32) {
    cmd.spawn((Sprite { color, custom_size: Some(Vec2::new(w, h)), ..default() }, Transform::from_xyz(x, y, z)));
}

fn lamp_post(cmd: &mut Commands, x: f32, y: f32) {
    // Pole
    rect(cmd, x, y - 10., 4., 36., Color::srgb(0.32, 0.30, 0.28), 1.1);
    // Head cap
    rect(cmd, x, y + 8., 14., 4., Color::srgb(0.28, 0.26, 0.24), 1.12);
    // Warm glow dot
    rect(cmd, x, y + 8., 8., 8., Color::srgba(1.0, 0.90, 0.55, 0.80), 1.15);
    // Collision (covers the pole)
    cmd.spawn((Transform::from_xyz(x, y - 10., 0.), Collider(Vec2::new(4., 18.))));
}

/// Spawns an invisible AABB collision wall (no visual).
fn wall(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32) {
    cmd.spawn((Transform::from_xyz(x, y, 0.), Collider(Vec2::new(w * 0.5, h * 0.5))));
}

fn zone(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, label: &str) {
    rect(cmd, x, y, w + 6., h + 6., Color::srgba(0., 0., 0., 0.50), 0.85);
    rect(cmd, x, y, w, h, color, 1.);
    cmd.spawn((Text2d::new(label), TextFont { font_size: 14., ..default() }, TextColor(Color::srgba(1.,1.,1.,0.50)), Transform::from_xyz(x, y+h/2.-16., 5.)));
}

fn obj(cmd: &mut Commands, x: f32, y: f32, w: f32, h: f32, color: Color, action: ActionKind, prompt: &str) {
    cmd.spawn((Sprite { color: Color::srgba(0., 0., 0., 0.45), custom_size: Some(Vec2::new(w + 4., h + 4.)), ..default() }, Transform::from_xyz(x + 2., y - 2., 1.95)));
    cmd.spawn((Sprite { color, custom_size: Some(Vec2::new(w, h)), ..default() }, Transform::from_xyz(x, y, 2.), Interactable { action, prompt: prompt.to_string() }, ObjectSize(Vec2::new(w, h))));
}

fn spawn_npc(cmd: &mut Commands, name: &str, npc_id: usize,
             outfit: Color, pants: Color, skin: Color, hair: Color,
             zone_center: Vec2, zone_half: f32, seed: u64) {
    let id = cmd.spawn((
        Transform::from_xyz(zone_center.x, zone_center.y, 9.),
        Visibility::default(),
        Npc { name: name.to_string(), wander_timer: 0., target: zone_center,
              zone_center, zone_half, rng: seed, velocity: Vec2::ZERO },
        Interactable { action: ActionKind::Chat, prompt: format!("[E] Chat with {}", name) },
        ObjectSize(Vec2::splat(18.)),
        NpcId(npc_id),
    )).with_children(|p| {
        spawn_human(p, outfit, pants, skin, hair);
    }).id();
    // Label floats above hair (hair tip at local y≈+15)
    cmd.spawn((Text2d::new(name), TextFont { font_size: 11., ..default() },
               TextColor(Color::WHITE),
               Transform::from_xyz(zone_center.x, zone_center.y + 26., 11.),
               NpcLabel(id)));
}

pub fn spawn_hud(cmd: &mut Commands) {
    cmd.spawn(Node { width: Val::Percent(100.), height: Val::Percent(100.), position_type: PositionType::Absolute, ..default() })
       .with_children(|root| {
        // Left panel
        root.spawn((
            Node { position_type: PositionType::Absolute, left: Val::Px(12.), top: Val::Px(12.), padding: UiRect::all(Val::Px(10.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(3.), ..default() },
            BackgroundColor(Color::srgba(0.,0.,0.,0.72)),
            BorderRadius::all(Val::Px(8.)),
        )).with_children(|p| {
            htxt(p, "Day 1  08:00 AM  (Mon)",               15., Color::srgb(1.,0.9,0.4),    HudLabel::Time);
            htxt(p, "$100 cash  savings $0  | 2 meals",     12., Color::srgb(0.4,1.,0.5),    HudLabel::Money);
            htxt(p, "",                                      11., Color::srgb(0.8,0.4,0.4),   HudLabel::Rent);
            htxt(p, "Mood: Happy | Junior | Apartment",     12., Color::srgb(0.8,0.9,1.0),   HudLabel::Mood);
            stat_bar(p, "Energy    ", Color::srgb(1.,0.78,0.2),  HudBar::Energy);
            stat_bar(p, "Hunger    ", Color::srgb(1.,0.44,0.2),  HudBar::Hunger);
            stat_bar(p, "Happiness ", Color::srgb(0.4,0.8, 1.),  HudBar::Happiness);
            stat_bar(p, "Health    ", Color::srgb(0.3,0.9, 0.4), HudBar::Health);
            stat_bar(p, "Stress    ", Color::srgb(0.9,0.3, 0.2), HudBar::Stress);
            htxt(p, "",                                      12., Color::srgb(1.,0.3,0.3),    HudLabel::Warning);
            htxt(p, "Streak: 0 days  Loan: $0",             11., Color::srgb(0.9,0.7,0.4),   HudLabel::Streak);
            p.spawn((Text::new("-- Skills --"), TextFont { font_size:11., ..default() }, TextColor(Color::srgb(0.5,0.5,0.5))));
            htxt(p, "Cook 0.0   Career 0.0\nFit  0.0   Social 0.0", 11., Color::srgb(0.75,0.85,1.0), HudLabel::Skills);
            p.spawn((Text::new("-- Friends --"), TextFont { font_size:11., ..default() }, TextColor(Color::srgb(0.5,0.5,0.5))));
            htxt(p, "Alex 0/5  Sam 0/5  Mia 0/5",           11., Color::srgb(1.0,0.65,0.75), HudLabel::Friendship);
            p.spawn((Text::new("-- Inventory --"), TextFont { font_size:11., ..default() }, TextColor(Color::srgb(0.5,0.5,0.5))));
            htxt(p, "Coffee x0  Vitamins x0  Books x0",     11., Color::srgb(0.9,0.8,0.6),   HudLabel::Inventory);
        });

        // Right panel
        root.spawn((
            Node { position_type: PositionType::Absolute, right: Val::Px(12.), top: Val::Px(12.), padding: UiRect::all(Val::Px(10.)), flex_direction: FlexDirection::Column, row_gap: Val::Px(4.), max_width: Val::Px(255.), ..default() },
            BackgroundColor(Color::srgba(0.,0.,0.,0.72)),
            BorderRadius::all(Val::Px(8.)),
        )).with_children(|p| {
            p.spawn((Text::new("-- Daily Goal --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.9,0.75,0.2))));
            htxt(p, "...", 12., Color::WHITE, HudLabel::Goal);
            p.spawn((Text::new("-- Weather --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.5,0.8,1.0))));
            htxt(p, "Sunny — outdoor bonus", 11., Color::srgb(1.0,0.95,0.6), HudLabel::Weather);
            p.spawn((Text::new("-- Hobbies --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.8,0.6,0.9))));
            htxt(p, "Paint 0.0  Game 0.0  Music 0.0", 11., Color::srgb(0.85,0.75,1.0), HudLabel::Hobbies);
            p.spawn((Text::new("-- Conditions --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.9,0.4,0.3))));
            htxt(p, "Healthy", 11., Color::srgb(0.4,0.9,0.5), HudLabel::Conditions);
            p.spawn((Text::new("-- Reputation --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.9,0.7,0.4))));
            htxt(p, "Rep: 0/100", 11., Color::srgb(1.0,0.85,0.5), HudLabel::Reputation);
            p.spawn((Text::new("-- Season --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.5,0.9,0.7))));
            htxt(p, "Spring — social bonus", 11., Color::srgb(0.7,1.0,0.8), HudLabel::Season);
            p.spawn((Text::new("-- Pet --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.9,0.75,0.5))));
            htxt(p, "No pet (adopt at Pet Bowl)", 11., Color::srgb(0.9,0.8,0.6), HudLabel::Pet);
            p.spawn((Text::new("-- Transport --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.7,0.7,0.85))));
            htxt(p, "On foot (buy at Garage)", 11., Color::srgb(0.8,0.8,0.95), HudLabel::Transport);
            p.spawn((Text::new("-- Life Rating --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.6,0.9,0.6))));
            htxt(p, "B — Comfortable", 13., Color::srgb(0.8,1.0,0.7), HudLabel::Rating);
            p.spawn((Text::new("-- Housing --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.75,0.65,0.5))));
            htxt(p, "Apartment | $20/day | [E] Bank to upgrade", 11., Color::srgb(0.82,0.76,0.65), HudLabel::Housing);
            p.spawn((Text::new("-- Milestones --"), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.9,0.85,0.3))));
            htxt(p, "None yet  (0/15)", 11., Color::srgb(0.95,0.90,0.6), HudLabel::Milestones);
        });

        // Notification center-top
        root.spawn(Node { position_type: PositionType::Absolute, top: Val::Px(12.), left: Val::Px(0.), right: Val::Px(0.), justify_content: JustifyContent::Center, ..default() })
            .with_children(|top| {
                top.spawn(Node { padding: UiRect::axes(Val::Px(16.), Val::Px(8.)), ..default() })
                   .with_children(|inner| {
                        inner.spawn((Text::new(""), TextFont { font_size:15., ..default() }, TextColor(Color::srgb(1.,0.88,0.3)), HudLabel::Notification));
                   });
            });

        // Prompt bottom-center
        root.spawn(Node { position_type: PositionType::Absolute, bottom: Val::Px(18.), left: Val::Px(0.), right: Val::Px(0.), justify_content: JustifyContent::Center, ..default() })
            .with_children(|b| {
                b.spawn((Node { padding: UiRect::axes(Val::Px(14.), Val::Px(7.)), ..default() }, BackgroundColor(Color::srgba(0.,0.,0.,0.72))))
                 .with_children(|inner| {
                    inner.spawn((Text::new(""), TextFont { font_size:15., ..default() }, TextColor(Color::WHITE), HudLabel::Prompt));
                 });
            });
    });
}

fn htxt(parent: &mut ChildBuilder, text: &str, size: f32, color: Color, label: HudLabel) {
    parent.spawn((Text::new(text), TextFont { font_size: size, ..default() }, TextColor(color), label));
}

fn stat_bar(parent: &mut ChildBuilder, label: &str, color: Color, bar: HudBar) {
    parent.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(5.), ..default() })
          .with_children(|row| {
        row.spawn((Text::new(label), TextFont { font_size:12., ..default() }, TextColor(Color::srgb(0.78,0.78,0.78))));
        row.spawn((Node { width: Val::Px(90.), height: Val::Px(9.), overflow: Overflow::clip(), ..default() }, BackgroundColor(Color::srgb(0.15,0.15,0.15)), BorderRadius::all(Val::Px(4.))))
           .with_children(|track| {
                track.spawn((Node { width: Val::Percent(80.), height: Val::Percent(100.), ..default() }, BackgroundColor(color), bar));
           });
    });
}
