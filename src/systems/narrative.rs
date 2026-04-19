use bevy::prelude::*;

use crate::resources::*;

#[allow(clippy::too_many_arguments)]
pub fn update_narrative(
    gt: Res<GameTime>,
    _gs: Res<GameState>,
    stats: Res<PlayerStats>,
    skills: Res<Skills>,
    friendship: Res<NpcFriendship>,
    housing: Res<HousingTier>,
    rating: Res<LifeRating>,
    conds: Res<Conditions>,
    rep: Res<Reputation>,
    transport: Res<Transport>,
    mut story: ResMut<NarrativeState>,
    mut notif: ResMut<Notification>,
) {
    let unlocked = (gt.day == 0)
        && story.unlock(
            "intro",
            "New in Town",
            "No home yet. Work to earn cash, then deposit $90 at the Bank to sign a lease. Short on energy? Rest at the park shelter for the night.",
        )
        || ((gt.day >= 1 || stats.money >= 140.)
            && story.unlock(
                "routine",
                "Finding a Rhythm",
                "The days are starting to rhyme. Work, meals, and rest are turning into a life.",
            ))
        || (friendship.levels.values().any(|&v| v >= 3.)
            && story.unlock(
                "friendship",
                "A Familiar Face",
                "One steady friendship makes the whole neighborhood feel less cold.",
            ))
        || (skills.career >= 2.5
            && story.unlock(
                "career",
                "A Door Opens",
                "Your effort is finally being noticed. Bigger opportunities may be ahead.",
            ))
        || (housing.has_access()
            && story.unlock(
                "home",
                "Room to Breathe",
                "You finally have a place to shut the door and call your own.",
            ))
        || (transport.kind.is_vehicle()
            && story.unlock(
                "wheels",
                "The Map Shrinks",
                "With wheels under you, the city suddenly feels smaller and possibility feels closer.",
            ))
        || ((conds.burnout || conds.malnourished || (stats.stress > 85. && stats.energy < 15.))
            && story.unlock(
                "strain",
                "Running on Empty",
                "You can push through anything for a while, but every life asks for balance in the end.",
            ))
        || ((rep.score >= 60. || stats.savings >= 250.)
            && story.unlock(
                "reputation",
                "The Neighborhood Notices",
                "People are starting to recognize your name. Your choices carry more weight now.",
            ))
        || ((rating.score >= 75. || skills.career >= 5.0 || *housing == HousingTier::Penthouse)
            && story.unlock(
                "legacy",
                "More Than Survival",
                "This is no longer just survival. Bit by bit, you are building a life with shape and meaning.",
            ));

    if unlocked && notif.timer <= 0. {
        if gt.day == 0 {
            notif.message = format!("Story: {} - [NE]Office  [E]Shop/Cafe  [SW]Bank  [N]Park+shelter", story.current_title);
            notif.timer = 9.;
        } else {
            notif.message = format!("Story: {}", story.current_title);
            notif.timer = 5.;
        }
    }
}
