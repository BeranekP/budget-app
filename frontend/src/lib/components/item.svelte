<script>
  export let name
  export let description
  export let item_type
  export let category
  export let amount
  export let date
  export let category_id
  export let type_id
  export let id

  const options = { year: "2-digit", month: "numeric", day: "numeric" }
  date = new Date(date).toLocaleDateString(undefined, options)
  amount = Math.round(amount)
  if (item_type === "expense") {
    amount = -amount
  }
</script>

<div class="item" {id}>
  <div class="subitem date">{date}</div>
  <div class="subitem amount {item_type}">{amount}</div>
  <div class="subitem category">{category}</div>
  <!--<div class="subitem desc hidden">{description}</div>-->
</div>

<style>
  .item {
    color: rgba(255, 255, 255, 0.7);
    background: rgba(255, 255, 255, 0.12);
    border: 1px solid rgba(255, 255, 255, 0.2);
    height: 3rem;
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    align-items: center;
    border-radius: 5px;
    font-size: 14pt;
    font-family: sans-serif;
    padding: 0.5rem 1rem;
  }

  .amount {
    text-align: right;
    padding-right: 2rem;
  }
  .expense {
    color: #c84b31;
  }
  .income {
    color: #4e9f3d;
  }
</style>
