<script lang="ts">
	import type { ServerMessage } from '#lib/protocol.js';

	let status = $state('disconnected');
	let log = $state<ServerMessage[]>([]);
</script>

<h1>Emergent Tactics</h1>
<p>Status: {status}</p>
<pre>{JSON.stringify(log, null, 2)}</pre>
