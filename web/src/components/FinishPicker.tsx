import { CheckIcon } from '@/components/icons';
import { type Finish, isLight } from '@/lib/finishes';

/**
 * The finishes Apple sold for this model, as swatches.
 *
 * Newer iOS builds no longer report the housing color, so for most modern phones this is how the
 * color gets set. Each swatch is a 32px target - comfortable for a finger - with the finish name as
 * its accessible name and tooltip, and the chosen one named beside the row.
 */
export function FinishPicker({ finishes, selected, onChange }: { finishes: Finish[]; selected: Finish | null; onChange: (finish: Finish | null) => void }) {
  return (
    <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
      <fieldset className="flex flex-wrap items-center gap-2">
        <legend className="sr-only">Color</legend>
        {finishes.map(finish => {
          const isSelected = selected?.name === finish.name;
          return (
            <button
              key={finish.name}
              type="button"
              aria-pressed={isSelected}
              aria-label={finish.name}
              title={finish.name}
              onClick={() => onChange(isSelected ? null : finish)}
              // The ring sits outside the swatch so the pigment itself is never tinted by the state.
              className={`relative size-8 rounded-full ring-1 ring-black/15 transition ring-inset dark:ring-white/25 ${
                isSelected
                  ? 'outline-2 outline-offset-2 outline-blue-600 dark:outline-blue-400'
                  : 'hover:outline-2 hover:outline-offset-2 hover:outline-slate-300 dark:hover:outline-slate-600'
              }`}
              style={{ backgroundColor: finish.hex }}>
              {isSelected && (
                <CheckIcon className="absolute inset-0 m-auto size-4" style={{ color: isLight(finish.hex) ? '#0f172a' : '#ffffff' }} aria-hidden="true" />
              )}
            </button>
          );
        })}
      </fieldset>
      <span className={`text-sm ${selected ? 'font-semibold text-slate-900 dark:text-slate-100' : 'text-slate-500 dark:text-slate-400'}`}>
        {selected ? selected.name : 'Pick the Color'}
      </span>
    </div>
  );
}
