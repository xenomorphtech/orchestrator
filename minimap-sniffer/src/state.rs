use std::collections::HashMap;

use crate::darkdec::{DecodedUpdate, EntityUpdate, PositionUpdate};

#[derive(Clone, Debug)]
pub struct Player {
    pub t: f64,
    pub x: f32,
    pub z: f32,
    pub rot: f32,
    pub updates: u64,
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub id: u8,
    pub t: f64,
    pub x: f32,
    pub z: f32,
    pub rot: f32,
    pub updates: u64,
    pub last_frame: u64,
}

#[derive(Default, Debug)]
pub struct Stats {
    pub packets: u64,
    pub tcp_payloads: u64,
    pub app_frames: u64,
    pub player_updates: u64,
    pub entity_updates: u64,
    pub bad_frames: u64,
}

#[derive(Default, Debug)]
pub struct WorldState {
    pub player: Option<Player>,
    pub entities: HashMap<u8, Entity>,
    pub stats: Stats,
}

impl WorldState {
    pub fn apply_update(&mut self, update: DecodedUpdate) -> String {
        match update {
            DecodedUpdate::Player(update) => self.apply_player(update),
            DecodedUpdate::Entity(update) => self.apply_entity(update),
        }
    }

    pub fn snapshot_json(&self) -> String {
        let mut entity_ids: Vec<u8> = self.entities.keys().copied().collect();
        entity_ids.sort_unstable();
        let entities = entity_ids
            .into_iter()
            .filter_map(|id| self.entities.get(&id))
            .map(entity_json)
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "{{\"type\":\"snapshot\",\"player\":{},\"entities\":[{}],\"stats\":{}}}",
            self.player
                .as_ref()
                .map(player_json)
                .unwrap_or_else(|| "null".to_string()),
            entities,
            stats_json(&self.stats)
        )
    }

    fn apply_player(&mut self, update: PositionUpdate) -> String {
        let updates = self.player.as_ref().map(|p| p.updates + 1).unwrap_or(1);
        self.stats.player_updates += 1;
        self.player = Some(Player {
            t: update.t,
            x: update.x,
            z: update.z,
            rot: update.rot,
            updates,
        });
        format!(
            "{{\"type\":\"player\",\"t\":{},\"x\":{},\"z\":{},\"rot\":{},\"updates\":{},\"frame\":{},\"stats\":{}}}",
            f64_json(update.t),
            f32_json(update.x),
            f32_json(update.z),
            f32_json(update.rot),
            updates,
            update.frame_ordinal,
            stats_json(&self.stats)
        )
    }

    fn apply_entity(&mut self, update: EntityUpdate) -> String {
        self.stats.entity_updates += 1;
        let updates = self
            .entities
            .get(&update.id)
            .map(|entity| entity.updates + 1)
            .unwrap_or(1);
        let entity = Entity {
            id: update.id,
            t: update.t,
            x: update.x,
            z: update.z,
            rot: update.rot,
            updates,
            last_frame: update.frame_ordinal,
        };
        self.entities.insert(update.id, entity);
        format!(
            "{{\"type\":\"entity\",\"id\":{},\"hex\":\"0x{:02x}\",\"t\":{},\"x\":{},\"z\":{},\"rot\":{},\"updates\":{},\"frame\":{},\"stats\":{}}}",
            update.id,
            update.id,
            f64_json(update.t),
            f32_json(update.x),
            f32_json(update.z),
            f32_json(update.rot),
            updates,
            update.frame_ordinal,
            stats_json(&self.stats)
        )
    }
}

fn player_json(player: &Player) -> String {
    format!(
        "{{\"t\":{},\"x\":{},\"z\":{},\"rot\":{},\"updates\":{}}}",
        f64_json(player.t),
        f32_json(player.x),
        f32_json(player.z),
        f32_json(player.rot),
        player.updates
    )
}

fn entity_json(entity: &Entity) -> String {
    format!(
        "{{\"id\":{},\"hex\":\"0x{:02x}\",\"t\":{},\"x\":{},\"z\":{},\"rot\":{},\"updates\":{},\"frame\":{}}}",
        entity.id,
        entity.id,
        f64_json(entity.t),
        f32_json(entity.x),
        f32_json(entity.z),
        f32_json(entity.rot),
        entity.updates,
        entity.last_frame
    )
}

fn stats_json(stats: &Stats) -> String {
    format!(
        "{{\"packets\":{},\"tcp_payloads\":{},\"app_frames\":{},\"player_updates\":{},\"entity_updates\":{},\"bad_frames\":{}}}",
        stats.packets,
        stats.tcp_payloads,
        stats.app_frames,
        stats.player_updates,
        stats.entity_updates,
        stats.bad_frames
    )
}

fn f32_json(value: f32) -> String {
    if value.is_finite() {
        format!("{value:.5}")
    } else {
        "null".to_string()
    }
}

fn f64_json(value: f64) -> String {
    if value.is_finite() {
        format!("{value:.5}")
    } else {
        "null".to_string()
    }
}
