<script>
  import { onMount } from "svelte"
  import Item from "$lib/components/item.svelte"

  let items = []
  const endpoint = "http://127.0.0.1:8000/items"

  onMount(async () => {
    const response = await fetch(endpoint)
    items = await response.json()
  })

  for (item of items) {
    console.log(item)
  }
</script>

<div class="container">
  {#each items as item}
    <Item {...item} />
  {:else}
    <p>Errro</p>
  {/each}
</div>

<style>
  .container {
    margin: 2rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>
