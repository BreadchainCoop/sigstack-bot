<script lang="ts">
	import type { CommandRow } from '$lib/content/types';

	let { rows }: { rows: CommandRow[] } = $props();
</script>

<div class="table-wrap">
	<table class="commands">
		<thead>
			<tr>
				<th scope="col">Command</th>
				<th scope="col">Where</th>
				<th scope="col">Effect</th>
			</tr>
		</thead>
		<tbody>
			{#each rows as row (row.command)}
				<tr>
					<td><code>{row.command}</code></td>
					<td>{row.where}</td>
					<td>{row.effect}</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>
