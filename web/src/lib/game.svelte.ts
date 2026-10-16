// Connection and game state, as Svelte 5 runes in a class so the page
// stays declarative.

import type {
	Action,
	CardInfo,
	CardKind,
	ClientMessage,
	Observation,
	Outcome,
	Player,
	ServerMessage,
	StatChange
} from './protocol.js';

/// One applied action, kept in the play zone for the rest of the turn.
export interface Play {
	id: number;
	player: Player;
	action: Action;
	changes: StatChange[];
}

/// The play zone's view of the turn in progress (or just ended).
export interface TurnView {
	actor: Player | null;
	plays: Play[];
	/// Set once the actor ended the turn; the plays linger until the next
	/// card is played by anyone.
	ended: Play | null;
}

export interface LogEntry {
	id: number;
	kind: 'ai' | 'act' | 'info' | 'error';
	player?: Player;
	title: string;
	lines: string[];
	decision?: Extract<ServerMessage, { type: 'ai_decision' }>;
}

export class GameClient {
	status = $state<'connecting' | 'open' | 'closed'>('connecting');
	cards = $state<Map<CardKind, CardInfo>>(new Map());
	difficulties = $state<string[]>([]);
	you = $state<Player | null>(null);
	seats = $state<{ first: string; second: string } | null>(null);
	observation = $state<Observation | null>(null);
	legal = $state<Action[]>([]);
	outcome = $state<Outcome | null>(null);
	log = $state<LogEntry[]>([]);
	thinking = $state(false);
	turn = $state<TurnView>({ actor: null, plays: [], ended: null });

	#ws: WebSocket | null = null;
	#nextId = 1;

	connect(url: string) {
		this.status = 'connecting';
		const ws = new WebSocket(url);
		this.#ws = ws;
		ws.onopen = () => (this.status = 'open');
		ws.onclose = () => (this.status = 'closed');
		ws.onmessage = (ev) => this.#handle(JSON.parse(ev.data) as ServerMessage);
	}

	send(msg: ClientMessage) {
		this.#ws?.send(JSON.stringify(msg));
	}

	newGame(difficulty: string, seat: Player) {
		this.#reset();
		this.send({ type: 'new_game', difficulty, seat });
	}

	spectate(first: string, second: string) {
		this.#reset();
		this.send({ type: 'spectate', first, second });
	}

	act(action: Action) {
		this.legal = [];
		this.send({ type: 'act', action });
	}

	get myTurn(): boolean {
		return this.you !== null && this.observation?.to_move === this.you && this.outcome === null;
	}

	#reset() {
		this.observation = null;
		this.legal = [];
		this.outcome = null;
		this.log = [];
		this.thinking = false;
		this.turn = { actor: null, plays: [], ended: null };
	}

	#record(play: Play) {
		const t = this.turn;
		if (play.action === 'EndTurn') {
			this.turn = { actor: play.player, plays: t.actor === play.player ? t.plays : [], ended: play };
			return;
		}
		const fresh = t.ended !== null || t.actor !== play.player;
		this.turn = { actor: play.player, plays: fresh ? [play] : [...t.plays, play], ended: null };
	}

	#push(entry: Omit<LogEntry, 'id'>) {
		this.log = [{ id: this.#nextId++, ...entry }, ...this.log].slice(0, 200);
	}

	#handle(msg: ServerMessage) {
		switch (msg.type) {
			case 'catalog':
				this.cards = new Map(msg.cards.map((c) => [c.kind, c]));
				this.difficulties = msg.difficulties;
				break;
			case 'game_started':
				this.you = msg.you;
				this.seats = { first: msg.first, second: msg.second };
				this.#push({
					kind: 'info',
					title: `Game ${msg.game_id.slice(0, 8)} started`,
					lines: [`First: ${msg.first}`, `Second: ${msg.second}`]
				});
				break;
			case 'state':
				this.observation = msg.observation;
				this.legal = msg.legal;
				this.thinking = this.you !== null && msg.observation.to_move !== this.you;
				break;
			case 'ai_decision':
				this.#push({
					kind: 'ai',
					player: msg.player,
					title: `${msg.player} (${msg.difficulty}) decided in ${msg.elapsed_ms} ms`,
					lines: [],
					decision: msg
				});
				break;
			case 'acted':
				this.#record({ id: this.#nextId, player: msg.player, action: msg.action, changes: msg.changes });
				this.#push({ kind: 'act', player: msg.player, title: msg.summary[0] ?? '', lines: msg.summary.slice(1) });
				break;
			case 'game_over':
				this.outcome = msg.outcome;
				this.thinking = false;
				this.#push({ kind: 'info', title: 'Game over', lines: [describeOutcome(msg.outcome, this.you)] });
				break;
			case 'error':
				this.#push({ kind: 'error', title: msg.message, lines: [] });
				break;
		}
	}
}

export function describeOutcome(o: Outcome, you: Player | null): string {
	if (o === 'Draw') return 'Draw';
	if (you === null) return `${o.Win} wins`;
	return o.Win === you ? 'You win' : 'You lose';
}
