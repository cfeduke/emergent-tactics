// Mirrors crates/et-protocol/src/ws.rs and the engine's public types.
// Keep in sync by hand for now; generating these from Rust (ts-rs) is a
// planned improvement.

export type Player = 'First' | 'Second';
export type CardKind = 'Strike' | 'Defend' | 'StandGround' | 'Charge' | 'Fury';
export type CardType = 'Attack' | 'Skill' | 'Power';
export type Archetype = 'All' | 'Warrior';
export type Action = { Play: CardKind } | 'EndTurn';
export type Outcome = { Win: Player } | 'Draw';

export interface CardInfo {
	kind: CardKind;
	name: string;
	card_type: CardType;
	archetype: Archetype;
	cost: number;
	text: string;
}

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

export interface CandidateInfo {
	action: Action;
	prior: number;
	visits: number;
	value: number;
}

export type ClientMessage =
	| { type: 'new_game'; difficulty: string; seat: Player }
	| { type: 'spectate'; first: string; second: string }
	| { type: 'act'; action: Action };

export type ServerMessage =
	| { type: 'catalog'; cards: CardInfo[]; difficulties: string[] }
	| { type: 'game_started'; game_id: string; you: Player | null; first: string; second: string }
	| { type: 'state'; observation: Observation; legal: Action[] }
	| {
			type: 'ai_decision';
			player: Player;
			difficulty: string;
			iterations: number;
			elapsed_ms: number;
			candidates: CandidateInfo[];
			chosen: Action;
	  }
	| { type: 'acted'; player: Player; action: Action; summary: string[] }
	| { type: 'game_over'; outcome: Outcome }
	| { type: 'error'; message: string };

export function actionLabel(a: Action, cards?: Map<CardKind, CardInfo>): string {
	if (a === 'EndTurn') return 'End Turn';
	return cards?.get(a.Play)?.name ?? a.Play;
}

export function sameAction(a: Action, b: Action): boolean {
	if (a === 'EndTurn' || b === 'EndTurn') return a === b;
	return a.Play === b.Play;
}
