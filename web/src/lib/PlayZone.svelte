<script lang="ts">
	import type { Play, TurnView } from './game.svelte.js';
	import type { CardInfo, CardKind, Player, StatChange } from './protocol.js';
	import Card from './Card.svelte';

	let {
		turn,
		cards,
		you,
		thinking = false
	}: { turn: TurnView; cards: Map<CardKind, CardInfo>; you: Player | null; thinking?: boolean } = $props();

	const who = (p: Player) => (you === null ? p : p === you ? 'You' : 'Opponent');

	interface Badge {
		text: string;
		tone: 'good' | 'bad' | 'neutral';
	}

	function badge(c: StatChange): Badge | null {
		const d = c.after - c.before;
		const sign = d > 0 ? '+' : '';
		switch (c.stat) {
			case 'life':
				return { text: `${sign}${d} ♥`, tone: d < 0 ? 'bad' : 'good' };
			case 'block':
				return { text: `${sign}${d} ⛨`, tone: d < 0 ? 'neutral' : 'good' };
			case 'vulnerable':
				return { text: `Vulnerable ${c.after}`, tone: 'bad' };
			case 'stand_ground':
				return d > 0 ? { text: 'Stand Ground readied', tone: 'good' } : null;
			case 'fury':
				return c.after ? { text: 'Fury in play', tone: 'good' } : null;
			case 'resource':
				return { text: `◆ ${c.after}`, tone: 'neutral' };
			default:
				return null;
		}
	}

	/// Badges for one play, grouped by affected entity, actor first.
	function effects(play: Play): { player: Player; badges: Badge[] }[] {
		const by = new Map<Player, Badge[]>();
		for (const c of play.changes) {
			const b = badge(c);
			if (!b) continue;
			if (!by.has(c.player)) by.set(c.player, []);
			by.get(c.player)!.push(b);
		}
		const order: Player[] = play.player === 'First' ? ['First', 'Second'] : ['Second', 'First'];
		return order.filter((p) => by.has(p)).map((p) => ({ player: p, badges: by.get(p)! }));
	}

	interface Stack {
		id: number;
		kind: CardKind;
		plays: Play[];
	}

	/// Consecutive plays of the same card collapse into one stack.
	const stacks = $derived.by((): Stack[] => {
		const out: Stack[] = [];
		for (const p of turn.plays) {
			if (p.action === 'EndTurn') continue;
			const last = out[out.length - 1];
			if (last && last.kind === p.action.Play) last.plays.push(p);
			else out.push({ id: p.id, kind: p.action.Play, plays: [p] });
		}
		return out;
	});

	const endBadges = $derived(turn.ended ? effects(turn.ended) : []);
</script>

<div class="zone" class:thinking class:ended={turn.ended !== null}>
	{#if turn.actor === null}
		<span class="empty">{thinking ? 'AI thinking…' : 'play zone'}</span>
	{:else}
		<div class="actor">{who(turn.actor)}</div>
		<div class="sequence">
			{#each stacks as s (s.id)}
				{@const n = s.plays.length}
				{@const latest = s.plays[n - 1]}
				<div class="entry">
					{#key latest.id}
						<div class="stack" style="--n: {Math.min(n, 4)}">
							{#each Array.from({ length: Math.min(n, 4) - 1 }) as _, i (i)}
								<div class="ghost" style="--i: {i + 1}"></div>
							{/each}
							<div class="top">
								<Card info={cards.get(s.kind)} />
							</div>
							{#if n > 1}<span class="count">×{n}</span>{/if}
						</div>
					{/key}
					<div class="effects">
						{#each s.plays as p (p.id)}
							<div class="play-effects">
								{#each effects(p) as g (g.player)}
									<span class="target">{who(g.player)}</span>
									{#each g.badges as b, i (i)}
										<span class="badge {b.tone}">{b.text}</span>
									{/each}
								{/each}
								{#if effects(p).length === 0}
									<span class="badge neutral">no visible change</span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/each}
			{#if turn.ended}
				{#key turn.ended.id}
					<div class="entry end">
						<div class="end-card">ended the turn</div>
						<div class="effects">
							{#each endBadges as g (g.player)}
								<div class="play-effects">
									<span class="target">{who(g.player)}</span>
									{#each g.badges as b, i (i)}
										<span class="badge {b.tone}">{b.text}</span>
									{/each}
								</div>
							{/each}
						</div>
					</div>
				{/key}
			{/if}
		</div>
	{/if}
</div>

<style>
	.zone {
		min-height: 11rem;
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 0.6rem 1rem;
		border: 1px dashed #2b3038;
		border-radius: 8px;
		background: radial-gradient(ellipse at center, #1c2129 0%, #15181d 70%);
		overflow-x: auto;
	}
	.zone.thinking {
		border-color: #f0c674;
	}
	.zone.ended .sequence {
		opacity: 0.75;
	}
	.actor {
		width: 5.5rem;
		flex: none;
		text-align: right;
		color: #aab0bb;
		font-size: 0.85rem;
	}
	.sequence {
		display: flex;
		align-items: flex-start;
		gap: 1.4rem;
		transition: opacity 0.3s;
	}
	.entry {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.4rem;
		animation: enter 0.35s ease-out;
	}
	@keyframes enter {
		from {
			opacity: 0;
			transform: translateY(14px) scale(0.96);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	.stack {
		position: relative;
		/* room for the offset ghosts behind the top card */
		padding-right: calc((var(--n) - 1) * 6px);
		padding-top: calc((var(--n) - 1) * 6px);
	}
	.ghost {
		position: absolute;
		inset: 0;
		top: calc((var(--n) - 1) * 6px - var(--i) * 6px);
		right: calc((var(--n) - 1) * 6px - var(--i) * 6px);
		bottom: calc(var(--i) * 6px);
		left: calc(var(--i) * 6px);
		background: #1b1f25;
		border: 1px solid #3a3f48;
		border-radius: 6px;
	}
	.top {
		position: relative;
		animation: slam 0.3s ease-out;
	}
	@keyframes slam {
		from {
			transform: translate(-10px, -10px) rotate(-3deg);
			opacity: 0.4;
		}
		to {
			transform: none;
			opacity: 1;
		}
	}
	.count {
		position: absolute;
		top: -0.5rem;
		right: -0.5rem;
		background: #f0c674;
		color: #15181d;
		font-weight: 700;
		font-size: 0.8rem;
		padding: 0.1rem 0.45rem;
		border-radius: 999px;
		box-shadow: 0 0 0 2px #15181d;
	}
	.end-card {
		width: 9rem;
		min-height: 7rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid #3a3f48;
		border-radius: 6px;
		color: #8a909c;
		font-style: italic;
		box-sizing: border-box;
	}
	.effects {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		min-height: 1.4rem;
	}
	.play-effects {
		display: flex;
		align-items: center;
		gap: 0.3rem;
		flex-wrap: wrap;
		animation: pop 0.4s ease-out;
	}
	@keyframes pop {
		from {
			opacity: 0;
			transform: translateX(-8px);
		}
		to {
			opacity: 1;
			transform: none;
		}
	}
	.target {
		font-size: 0.7rem;
		color: #8a909c;
	}
	.badge {
		padding: 0.1rem 0.45rem;
		border-radius: 999px;
		font-size: 0.8rem;
		font-weight: 600;
		background: #2b3038;
	}
	.badge.bad {
		color: #e06c75;
		background: #3a2326;
	}
	.badge.good {
		color: #98c379;
		background: #22321f;
	}
	.badge.neutral {
		color: #aab0bb;
	}
	.empty {
		color: #4f5663;
		font-size: 0.85rem;
		margin: auto;
	}
</style>
