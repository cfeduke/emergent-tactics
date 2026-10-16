<script lang="ts">
	import type { LogEntry } from './game.svelte.js';
	import { sameAction, type Action } from './protocol.js';
	let { entries, label }: { entries: LogEntry[]; label: (a: Action) => string } = $props();
</script>

<h2>Log</h2>
{#each entries as e (e.id)}
	<article class={e.kind}>
		<header>{e.title}</header>
		{#if e.decision}
			<table>
				<thead>
					<tr><th>action</th><th>prior</th><th>visits</th><th>value</th></tr>
				</thead>
				<tbody>
					{#each e.decision.candidates as c (label(c.action))}
						<tr class:chosen={sameAction(c.action, e.decision.chosen)}>
							<td>{label(c.action)}</td>
							<td>{c.prior.toFixed(2)}</td>
							<td>{c.visits}</td>
							<td>{c.value.toFixed(2)}</td>
						</tr>
					{/each}
				</tbody>
			</table>
			<div class="small">{e.decision.iterations} iterations budgeted</div>
		{/if}
		{#each e.lines as line, i (i)}
			<div class="line">{line}</div>
		{/each}
	</article>
{/each}

<style>
	h2 {
		font-size: 0.9rem;
		margin: 0 0 0.5rem;
		color: #aab0bb;
	}
	article {
		font-size: 0.8rem;
		padding: 0.4rem 0.5rem;
		margin-bottom: 0.4rem;
		border-left: 3px solid #3a3f48;
		background: #1b1f25;
	}
	article.ai {
		border-color: #c678dd;
	}
	article.act {
		border-color: #61afef;
	}
	article.error {
		border-color: #e06c75;
	}
	header {
		font-weight: 600;
	}
	.line,
	.small {
		color: #aab0bb;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin: 0.3rem 0;
		font-variant-numeric: tabular-nums;
	}
	th,
	td {
		text-align: left;
		padding: 0.1rem 0.3rem;
	}
	th {
		color: #8a909c;
		font-weight: 500;
	}
	tr.chosen td {
		color: #98c379;
		font-weight: 600;
	}
</style>
