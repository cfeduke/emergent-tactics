<script lang="ts">
	import type { Play } from './game.svelte.js';
	import type { CardInfo, CardKind, Player, StatChange } from './protocol.js';
	import Card from './Card.svelte';

	let {
		play,
		cards,
		you,
		thinking = false
	}: { play: Play | null; cards: Map<CardKind, CardInfo>; you: Player | null; thinking?: boolean } = $props();

	const who = (p: Player) => (you === null ? p : p === you ? 'You' : 'Opponent');

	function badge(c: StatChange): { text: string; tone: 'good' | 'bad' | 'neutral' } | null {
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

	/// Group badges by the entity they affected, actor first.
	const groups = $derived.by(() => {
		if (!play) return [];
		const by = new Map<Player, { text: string; tone: string }[]>();
		for (const c of play.changes) {
			const b = badge(c);
			if (!b) continue;
			if (!by.has(c.player)) by.set(c.player, []);
			by.get(c.player)!.push(b);
		}
		const order: Player[] = play.player === 'First' ? ['First', 'Second'] : ['Second', 'First'];
		return order.filter((p) => by.has(p)).map((p) => ({ player: p, badges: by.get(p)! }));
	});
</script>

<div class="zone" class:thinking>
	{#if play}
		{#key play.id}
			<div class="play">
				<div class="actor">{who(play.player)}</div>
				{#if play.action === 'EndTurn'}
					<div class="end">ended the turn</div>
				{:else}
					<Card info={cards.get(play.action.Play)} />
				{/if}
				<div class="effects">
					{#each groups as g (g.player)}
						<div class="group">
							<span class="target">{who(g.player)}</span>
							{#each g.badges as b, i (i)}
								<span class="badge {b.tone}">{b.text}</span>
							{/each}
						</div>
					{/each}
					{#if groups.length === 0 && play.action !== 'EndTurn'}
						<span class="badge neutral">no visible change</span>
					{/if}
				</div>
			</div>
		{/key}
	{:else}
		<span class="empty">{thinking ? 'AI thinking…' : 'play zone'}</span>
	{/if}
</div>

<style>
	.zone {
		min-height: 9.5rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px dashed #2b3038;
		border-radius: 8px;
		background: radial-gradient(ellipse at center, #1c2129 0%, #15181d 70%);
	}
	.zone.thinking {
		border-color: #f0c674;
	}
	.play {
		display: flex;
		align-items: center;
		gap: 1.2rem;
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
	.actor {
		width: 6rem;
		text-align: right;
		color: #aab0bb;
		font-size: 0.85rem;
	}
	.end {
		width: 9rem;
		min-height: 7rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px solid #3a3f48;
		border-radius: 6px;
		color: #8a909c;
		font-style: italic;
	}
	.effects {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		min-width: 10rem;
	}
	.group {
		display: flex;
		align-items: center;
		gap: 0.4rem;
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
		font-size: 0.75rem;
		color: #8a909c;
		width: 4.5rem;
	}
	.badge {
		padding: 0.15rem 0.5rem;
		border-radius: 999px;
		font-size: 0.85rem;
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
	}
</style>
