// Mirrors crates/et-protocol/src/ws.rs. Keep in sync by hand for now;
// generating these from Rust (ts-rs) is a planned improvement.

export type Player = 'First' | 'Second';
export type Action = 'Pass';
export type Outcome = { Win: Player } | 'Draw';

export interface Observation {
	viewer: Player;
	to_move: Player;
	turn: number;
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
