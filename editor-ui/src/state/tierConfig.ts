export interface TierVisibility {
  showHeaderToolbar: boolean;
  showLeftToolStrip: boolean;
  showLeftPanel: boolean;
  showTabBar: boolean;
  showBottomTabStrip: boolean;
  showStatusBar: boolean;
  showRightToolStrip: boolean;
  showCrtEffects: boolean;
}

export function getTierVisibility(tier: number): TierVisibility {
  return {
    showHeaderToolbar: tier >= 1,
    showLeftToolStrip: tier >= 1,
    showLeftPanel: tier >= 1,
    showTabBar: tier >= 1,
    showBottomTabStrip: tier >= 1,
    showStatusBar: tier >= 1,
    showRightToolStrip: tier >= 2,
    showCrtEffects: tier === 0,
  };
}
