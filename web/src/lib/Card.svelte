<script lang="ts">
	import type { CardInfo } from './protocol.js';
	let { info, playable = false, onplay }: { info?: CardInfo; playable?: boolean; onplay?: () => void } = $props();
</script>

<button class="card {info?.card_type.toLowerCase()}" class:playable disabled={!playable} onclick={onplay}>
	<span class="cost">{info?.cost ?? '?'}</span>
	<span class="name">{info?.name ?? 'Unknown'}</span>
	<span class="type">{info?.card_type}</span>
	<span class="text">{info?.text}</span>
</button>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		width: 9rem;
		min-height: 7rem;
		padding: 0.5rem;
		text-align: left;
		background: #22262d;
		border: 1px solid #3a3f48;
		border-radius: 6px;
		color: #e7e9ee;
		cursor: default;
		position: relative;
	}
	.card.playable {
		border-color: #5aa0ff;
		cursor: pointer;
	}
	.card.playable:hover {
		background: #2a3140;
	}
	.card:disabled {
		opacity: 0.65;
	}
	.cost {
		position: absolute;
		top: 0.3rem;
		right: 0.4rem;
		font-weight: 700;
		color: #f0c674;
	}
	.name {
		font-weight: 600;
	}
	.type {
		font-size: 0.7rem;
		text-transform: uppercase;
		color: #8a909c;
	}
	.attack .type {
		color: #e06c75;
	}
	.skill .type {
		color: #61afef;
	}
	.power .type {
		color: #c678dd;
	}
	.text {
		font-size: 0.72rem;
		color: #aab0bb;
	}
</style>
