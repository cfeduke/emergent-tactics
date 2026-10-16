// Mirrors crates/et-protocol/src/ws.rs. Keep in sync by hand for now;
// generating these from Rust (ts-rs) is a planned improvement.

export type Player = 'First' | 'Second';
export type CardKind = 'Strike' | 'Defend' | 'StandGround' | 'Charge' | 'Fury';
export type Action = { Play: CardKind } | 'EndTurn';
export type Outcome = { Win: Player } | 'Draw';

export type Archetype = 'All' | 'Warrior';

export interface PublicEntity {
	archetype: Archetype;
	lifepool: number;
	block: number;
	resource: number;
	resource_limit: number;
	resource_bonus: number;
	vulnerable: number;
	fury: boolean;
	stand_ground_pending: number;
	hand_count: number;
	draw_count: number;
	discard_face_up: CardKind[];
	discard_face_down_count: number;
	exile: CardKind[];
	played_this_turn: CardKind[];
}

export interface Observation {
	viewer: Player;
	to_move: Player;
	round: number;
	turn: number;
	me: PublicEntity;
	opponent: PublicEntity;
	hand: CardKind[];
	discard_face_down: CardKind[];
}

export type ClientMessage =
	| { type: 'new_game'; difficulty: string }
	| { type: 'spectate'; first: string; second: string }
	| { type: 'act'; action: Action };

export type ServerMessage =
	| { type: 'game_started'; game_id: string; you: Player | null }
	| { type: 'state'; observation: Observation; legal: Action[] }
	| { type: 'thinking'; player: Player; iterations: number; visits: [Action, number][] }
	| { type: 'acted'; player: Player; action: Action }
	| { type: 'game_over'; outcome: Outcome }
	| { type: 'error'; message: string };
