import { CompletionContext, type CompletionResult, type Completion } from '@codemirror/autocomplete';
import { useStore } from '../state/store';

const keywordCompletions: Completion[] = [
  'while', 'if', 'else', 'elif', 'for', 'in', 'def', 'return',
  'and', 'or', 'not', 'break', 'continue', 'pass', 'True', 'False', 'None',
].map((label) => ({ label, type: 'keyword' }));

const stdlibCompletions: Completion[] = [
  { label: 'print', detail: '(msg)', info: 'Print to console' },
  { label: 'len', detail: '(obj)', info: 'Get length' },
  { label: 'range', detail: '([start,] end [, step])', info: 'Generate range' },
  { label: 'abs', detail: '(n)', info: 'Absolute value' },
  { label: 'min', detail: '(a, b)', info: 'Minimum of two values' },
  { label: 'max', detail: '(a, b)', info: 'Maximum of two values' },
  { label: 'int', detail: '(x)', info: 'Convert to integer' },
  { label: 'str', detail: '(x)', info: 'Convert to string' },
  { label: 'type', detail: '(x)', info: 'Get type name' },
].map((c) => ({ ...c, type: 'function' }));

const gameCommandCompletions: Completion[] = [
  { label: 'move', detail: '(position)', info: 'Move toward a position' },
  { label: 'get_pos', detail: '([entity])', info: 'Get position' },
  { label: 'scan', detail: '(type)', info: 'Scan for entities of type' },
  { label: 'nearest', detail: '(type)', info: 'Find nearest entity of type' },
  { label: 'distance', detail: '(a, b)', info: 'Get distance between entities' },
  { label: 'attack', detail: '(target)', info: 'Attack a target' },
  { label: 'flee', detail: '(threat)', info: 'Flee from threat' },
  { label: 'wait', detail: '()', info: 'Wait one tick' },
  { label: 'set_target', detail: '(target)', info: 'Set current target' },
  { label: 'get_target', detail: '()', info: 'Get current target' },
  { label: 'has_target', detail: '()', info: 'Check if target is set' },
  { label: 'get_health', detail: '([entity])', info: 'Get health' },
  { label: 'get_energy', detail: '([entity])', info: 'Get energy' },
  { label: 'get_shield', detail: '([entity])', info: 'Get shield' },
  { label: 'get_type', detail: '(entity)', info: 'Get entity type' },
  { label: 'get_name', detail: '(entity)', info: 'Get entity name' },
  { label: 'get_owner', detail: '(entity)', info: 'Get entity owner' },
  { label: 'consult', detail: '()', info: 'Consult the spirits' },
  { label: 'raise', detail: '()', info: 'Raise the dead' },
  { label: 'harvest', detail: '()', info: 'Harvest essence' },
  { label: 'pact', detail: '()', info: 'Forge a dark pact' },
].map((c) => ({ ...c, type: 'function' }));

export function grimScriptCompletion(context: CompletionContext): CompletionResult | null {
  const word = context.matchBefore(/\w*/);
  if (!word || (word.from === word.to && !context.explicit)) return null;

  const state = useStore.getState();
  const available = new Set(state.availableCommands);
  const filteredGameCommands = gameCommandCompletions.filter((c) => available.has(c.label));

  // Build dynamic completions from command_info (custom mod commands).
  const customCompletions: Completion[] = state.commandInfo
    .filter((ci) => available.has(ci.name))
    .filter((ci) => !gameCommandCompletions.some((gc) => gc.label === ci.name))
    .map((ci) => ({
      label: ci.name,
      detail: ci.args.length > 0 ? `(${ci.args.join(', ')})` : '()',
      info: ci.description,
      type: 'function' as const,
    }));

  return {
    from: word.from,
    options: [...keywordCompletions, ...stdlibCompletions, ...filteredGameCommands, ...customCompletions],
    validFor: /^\w*$/,
  };
}
