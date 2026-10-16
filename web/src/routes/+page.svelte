<script lang="ts">
	import { onMount } from 'svelte';
	import { GameClient } from '#lib/game.svelte.js';
	import { actionLabel, sameAction, type Action, type Player, type CardKind } from '#lib/protocol.js';
	import Entity from '#lib/Entity.svelte';
	import Card from '#lib/Card.svelte';
	import DecisionLog from '#lib/DecisionLog.svelte';

	const client = new GameClient();

	let difficulty = $state('novice');
	let seat = $state<Player>('First');
	let first = $state('novice');
	let second = $state('novice');

	onMount(() => {
		const proto = location.protocol === 'https:' ? 'wss' : 'ws';
		client.connect(`${proto}://${location.host}/ws`);
	});

	const obs = $derived(client.observation);
	const canPlay = (kind: CardKind) => client.legal.some((a) => sameAction(a, { Play: kind }));
	const canEnd = $derived(client.legal.some((a) => a === 'EndTurn'));

	function play(kind: CardKind) {
		if (canPlay(kind)) client.act({ Play: kind });
	}
	function endTurn() {
		if (canEnd) client.act('EndTurn');
	}
	const label = (a: Action) => actionLabel(a, client.cards);
</script>

<svelte:head><title>Emergent Tactics</title></svelte:head>

<header>
	<h1>Emergent Tactics</h1>
	<span class="status {client.status}">{client.status}</span>
	<form class="controls" onsubmit={(e) => e.preventDefault()}>
		<label>
			Play vs
			<select bind:value={difficulty}>
				{#each client.difficulties as d (d)}<option value={d}>{d}</option>{/each}
			</select>
		</label>
		<label>
			as
			<select bind:value={seat}>
				<option value="First">First</option>
				<option value="Second">Second</option>
			</select>
		</label>
		<button onclick={() => client.newGame(difficulty, seat)} disabled={client.status !== 'open'}>New game</button>
		<span class="sep"></span>
		<label>
			Spectate
			<select bind:value={first}>
				{#each client.difficulties as d (d)}<option value={d}>{d}</option>{/each}
			</select>
		</label>
		<label>
			vs
			<select bind:value={second}>
				{#each client.difficulties as d (d)}<option value={d}>{d}</option>{/each}
			</select>
		</label>
		<button onclick={() => client.spectate(first, second)} disabled={client.status !== 'open'}>Watch</button>
	</form>
</header>

<main>
	<section class="board">
		{#if obs}
			<div class="meta">
				Round {obs.round + 1} · Turn {obs.turn + 1} · {obs.to_move} to move
				{#if client.seats}
					· First: {client.seats.first} · Second: {client.seats.second}
				{/if}
				{#if client.you}· You are {client.you}{/if}
				{#if client.thinking}<em class="thinking"> · AI thinking…</em>{/if}
			</div>

			<Entity entity={obs.opponent} title={client.you ? 'Opponent' : obs.viewer === 'First' ? 'Second' : 'First'} cards={client.cards} active={obs.to_move !== obs.viewer} />

			<div class="hand-row">
				<div class="hand">
					{#each obs.hand as kind, i (i)}
						<Card info={client.cards.get(kind)} playable={canPlay(kind)} onplay={() => play(kind)} />
					{/each}
					{#if obs.hand.length === 0}<span class="empty">empty hand</span>{/if}
				</div>
				<button class="end" onclick={endTurn} disabled={!canEnd}>End turn</button>
			</div>

			<Entity entity={obs.me} title={client.you ? 'You' : obs.viewer} cards={client.cards} active={obs.to_move === obs.viewer} faceDown={obs.discard_face_down} />

			{#if client.outcome}
				<div class="banner">
					{client.outcome === 'Draw' ? 'Draw' : `${client.outcome.Win} wins`}
				</div>
			{/if}
		{:else}
			<p class="empty">Start a game or spectate one.</p>
		{/if}
	</section>

	<aside>
		<DecisionLog entries={client.log} {label} />
	</aside>
</main>

<style>
	:global(body) {
		margin: 0;
		font-family: system-ui, sans-serif;
		background: #15181d;
		color: #e7e9ee;
	}
	header {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 1rem;
		padding: 0.6rem 1rem;
		border-bottom: 1px solid #2b3038;
	}
	h1 {
		font-size: 1.1rem;
		margin: 0;
	}
	.status {
		font-size: 0.75rem;
		padding: 0.1rem 0.5rem;
		border-radius: 999px;
		background: #3a3f48;
	}
	.status.open {
		background: #1f6b3a;
	}
	.status.closed {
		background: #7a2b2b;
	}
	.controls {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-left: auto;
		font-size: 0.85rem;
	}
	.sep {
		width: 1px;
		height: 1.4rem;
		background: #2b3038;
	}
	main {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 380px;
		gap: 1rem;
		padding: 1rem;
		height: calc(100vh - 3.5rem);
		box-sizing: border-box;
	}
	.board {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		min-width: 0;
	}
	.meta {
		font-size: 0.85rem;
		color: #aab0bb;
	}
	.thinking {
		color: #f0c674;
	}
	.hand-row {
		display: flex;
		align-items: center;
		gap: 1rem;
	}
	.hand {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
		flex: 1;
		min-height: 7rem;
	}
	.end {
		padding: 0.6rem 1rem;
	}
	.empty {
		color: #6c7380;
	}
	.banner {
		font-size: 1.4rem;
		text-align: center;
		padding: 0.5rem;
		background: #2b3038;
		border-radius: 6px;
	}
	aside {
		min-height: 0;
		overflow: auto;
		border-left: 1px solid #2b3038;
		padding-left: 1rem;
	}
	button {
		background: #2f6fde;
		color: white;
		border: 0;
		border-radius: 4px;
		padding: 0.3rem 0.7rem;
		cursor: pointer;
	}
	button:disabled {
		background: #3a3f48;
		color: #8a909c;
		cursor: default;
	}
	select {
		background: #22262d;
		color: inherit;
		border: 1px solid #3a3f48;
		border-radius: 4px;
		padding: 0.2rem;
	}
</style>
