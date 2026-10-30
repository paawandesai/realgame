import { createComponent, Types } from '@iwsdk/core';

/** A reagent orb on the shelf. Dropped in the bowl, it adds one molecule. */
export const Reagent = createComponent('Reagent', {
  formula: { type: Types.String, default: 'H2' },
  /** Shelf slot the orb springs back to after each drop. */
  slot: { type: Types.Int16, default: 0 },
});

/** A pokeable bench button: `spark`, `zap` or `empty`. */
export const BenchButton = createComponent('BenchButton', {
  action: { type: Types.String, default: 'spark' },
});
