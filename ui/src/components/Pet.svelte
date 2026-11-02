<!-- The supplied Codex pet sheet has 8 columns and 9 rows of 192 × 208px frames. -->
<script lang="ts">
  import spritesheet from '../assets/pet/spritesheet.webp';
  import pet from '../assets/pet/pet.json';

  let {
    pressed = false,
    open = false,
    stir = false,
  }: { pressed?: boolean; open?: boolean; stir?: boolean } = $props();

  // Both chosen rows contain six frames. Leave unused cells out of playback.
  const row = $derived(open ? 6 : 0);
</script>

<span
  class="pet"
  class:pressed
  class:stir
  style:--row={row}
  data-pet={pet.displayName}
  aria-hidden="true"
>
  <img class="sheet" src={spritesheet} alt="" draggable="false" />
</span>

<style>
  .pet {
    position: relative;
    display: block;
    height: 100%;
    aspect-ratio: 192 / 208;
    overflow: hidden;
    pointer-events: none;
    transition: transform var(--dur-fast) var(--ease);
  }

  .sheet {
    position: absolute;
    left: 0;
    top: calc(var(--row) * -100%);
    width: 800%;
    height: 900%;
    max-width: none;
    user-select: none;
  }

  /* One short burst per interaction/idle timer; resting on frame zero avoids
     constantly repainting an always-on-top transparent desktop window. */
  .stir .sheet {
    animation: pet-frames 900ms steps(6, end) 1;
  }

  @keyframes pet-frames {
    from {
      transform: translateX(0);
    }
    to {
      transform: translateX(-75%);
    }
  }

  .pressed {
    transform: translateY(1px) scale(0.94);
  }

  @media (prefers-reduced-motion: reduce) {
    .pet {
      transition: none;
    }
    .stir .sheet {
      animation: none;
    }
  }
</style>
