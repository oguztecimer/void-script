import { CompletionContext, type CompletionResult, type Completion } from '@codemirror/autocomplete';

const keywordCompletions: Completion[] = [
  'while', 'if', 'else', 'elif', 'for', 'in', 'def', 'return',
  'and', 'or', 'not', 'break', 'continue', 'pass', 'True', 'False', 'None',
].map((label) => ({ label, type: 'keyword' }));

const constantCompletions: Completion[] = [
  'NORTH', 'SOUTH', 'EAST', 'WEST',
  'ASTEROID', 'MINER', 'FIGHTER', 'SCOUT', 'HAULER',
  'IRON', 'COPPER', 'SILICON', 'URANIUM', 'CRYSTAL',
].map((label) => ({ label, type: 'constant' }));

const functionCompletions: Completion[] = [
  { label: 'move', detail: '(direction)', info: 'Move the ship in a direction' },
  { label: 'mine', detail: '()', info: 'Mine the current asteroid' },
  { label: 'can_mine', detail: '()', info: 'Check if current location is mineable' },
  { label: 'deposit', detail: '()', info: 'Deposit cargo at a station' },
  { label: 'get_pos', detail: '()', info: 'Get current position (x, y)' },
  { label: 'scan', detail: '(radius?)', info: 'Scan nearby objects' },
  { label: 'get_cargo', detail: '()', info: 'Get cargo contents' },
  { label: 'cargo_full', detail: '()', info: 'Check if cargo is full' },
  { label: 'nearest', detail: '(type)', info: 'Find nearest object of type' },
  { label: 'distance', detail: '(target)', info: 'Get distance to target' },
  { label: 'attack', detail: '(target)', info: 'Attack a target' },
  { label: 'flee', detail: '(direction?)', info: 'Flee from combat' },
  { label: 'dock', detail: '(station)', info: 'Dock at a station' },
  { label: 'undock', detail: '()', info: 'Undock from station' },
  { label: 'build', detail: '(type)', info: 'Build a new ship' },
  { label: 'print', detail: '(msg)', info: 'Print to console' },
  { label: 'get_health', detail: '()', info: 'Get current health' },
  { label: 'get_energy', detail: '()', info: 'Get current energy' },
  { label: 'get_shield', detail: '()', info: 'Get current shield' },
  { label: 'wait', detail: '()', info: 'Wait one tick' },
  { label: 'set_target', detail: '(target)', info: 'Set navigation target' },
  { label: 'get_target', detail: '()', info: 'Get current target' },
  { label: 'has_target', detail: '()', info: 'Check if target is set' },
].map((c) => ({ ...c, type: 'function' }));

const allCompletions = [...keywordCompletions, ...constantCompletions, ...functionCompletions];

export function voidScriptCompletion(context: CompletionContext): CompletionResult | null {
  const word = context.matchBefore(/\w*/);
  if (!word || (word.from === word.to && !context.explicit)) return null;

  return {
    from: word.from,
    options: allCompletions,
    validFor: /^\w*$/,
  };
}
