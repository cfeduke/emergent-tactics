<script lang="ts">
	import type { CardInfo, CardKind, PublicEntity } from './protocol.js';
	let {
		entity,
		title,
		cards,
		active = false,
		faceDown
	}: { entity: PublicEntity; title: string; cards: Map<CardKind, CardInfo>; active?: boolean; faceDown?: CardKind[] } = $props();

	const name = (k: CardKind) => cards.get(k)?.name ?? k;
	const list = (ks: CardKind[]) => (ks.length ? ks.map(name).join(', ') : '—');
</script>

<div class="entity" class:active>
	<div class="head">
		<strong>{title}</strong>
		<span class="arch">{entity.archetype}</span>
		{#if active}<span class="turn">to move</span>{/if}
	</div>
	<div class="stats">
		<span class="stat life">♥ {entity.lifepool}</span>
		<span class="stat block">⛨ {entity.block}</span>
		<span class="stat res">◆ {entity.resource} / {entity.resource_limit}{entity.resource_bonus ? ` +${entity.resource_bonus}` : ''}</span>
		{#if entity.vulnerable}<span class="stat bad">Vulnerable {entity.vulnerable}</span>{/if}
		{#if entity.fury}<span class="stat good">Fury</span>{/if}
		{#if entity.stand_ground_pending}<span class="stat good">Stand Ground ×{entity.stand_ground_pending}</span>{/if}
	</div>
	<div class="zones">
		<span>Hand {entity.hand_count}</span>
		<span>Draw {entity.draw_count}</span>
		<span>Discard ↑ [{list(entity.discard_face_up)}]</span>
		<span>Discard ↓ {entity.discard_face_down_count}{faceDown ? ` [${list(faceDown)}]` : ''}</span>
		<span>Exile [{list(entity.exile)}]</span>
		<span>Played this turn [{list(entity.played_this_turn)}]</span>
	</div>
</div>

<style>
	.entity {
		padding: 0.6rem 0.8rem;
		border: 1px solid #2b3038;
		border-radius: 6px;
		background: #1b1f25;
	}
	.entity.active {
		border-color: #f0c674;
	}
	.head {
		display: flex;
		gap: 0.6rem;
		align-items: baseline;
	}
	.arch {
		color: #8a909c;
		font-size: 0.8rem;
	}
	.turn {
		margin-left: auto;
		color: #f0c674;
		font-size: 0.8rem;
	}
	.stats {
		display: flex;
		gap: 0.8rem;
		margin: 0.4rem 0;
		font-size: 1.05rem;
	}
	.life {
		color: #e06c75;
	}
	.block {
		color: #61afef;
	}
	.res {
		color: #f0c674;
	}
	.bad {
		color: #e06c75;
		font-size: 0.85rem;
	}
	.good {
		color: #98c379;
		font-size: 0.85rem;
	}
	.zones {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem 1rem;
		font-size: 0.78rem;
		color: #aab0bb;
	}
</style>
