use crate::resources::*;
use bevy::prelude::*;

/// Checks once per day whether a festival should start or end.
/// Festivals run on days 25-27 of each 30-day season.
pub fn festival_trigger_system(
    gt: Res<GameTime>,
    mut festival: ResMut<FestivalState>,
    season: Res<Season>,
    mut notif: ResMut<Notification>,
) {
    if gt.day == festival.last_check_day {
        return;
    }
    festival.last_check_day = gt.day;
    festival.activities_today = 0;

    let season_day = gt.day % 30;

    // Festival runs on days 25, 26, 27 of each season
    if (25..=27).contains(&season_day) {
        let kind = match &season.current {
            SeasonKind::Spring => FestivalKind::SpringFair,
            SeasonKind::Summer => FestivalKind::SummerBBQ,
            SeasonKind::Autumn => FestivalKind::AutumnHarvest,
            SeasonKind::Winter => FestivalKind::WinterGala,
        };

        if festival.active.is_none() {
            notif.push(
                format!("{} has begun! Visit the Park for special activities.", kind.label()),
                6.,
            );
        }

        festival.active = Some(kind);
    } else if let Some(kind) = festival.active.take() {
        notif.push(
            format!("{} is over! You earned {} tokens total.", kind.label(), festival.tokens),
            5.,
        );
    }
}
