import { useMemo } from 'react';
import { useStore } from './store';
import { getTierVisibility } from './tierConfig';

export function useTierVisibility() {
  const tier = useStore((s) => s.tier);
  return useMemo(() => getTierVisibility(tier), [tier]);
}
