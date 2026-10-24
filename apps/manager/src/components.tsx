import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import type { UsageWindow } from "./contract";
import { currentPercent, resetPassed } from "./freshness";
import { describeError, useCopy } from "./i18n";

const paths = {
  accounts:
    "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3a4 4 0 0 1 0 8m6 10v-2a4 4 0 0 0-3-3.87M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
  projects: "M3 7h6l2 2h10v11H3zM3 7V4h6l2 3",
  sessions:
    "M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.4 8.4 0 0 1 3.8-.9h.5a8.5 8.5 0 0 1 8 8z",
  desktop: "M3 4h18v12H3zM8 20h8M12 16v4",
  refresh: "M20 7v5h-5M4 17v-5h5M6 7a7 7 0 0 1 12-2l2 3M4 16l2 3a7 7 0 0 0 12-2",
  arrow: "M7 17 17 7M7 7h10v10",
  plus: "M12 5v14M5 12h14",
};

export function Icon({ name }: { name: keyof typeof paths }) {
  return (
    <svg
      aria-hidden="true"
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d={paths[name]} />
    </svg>
  );
}

const clamp = (n: number) => Math.max(0, Math.min(100, n));
// The copy's locale, not the browser's: dates follow the same language as
// every other string.
const when = (locale: string, epoch: number) =>
  new Date(epoch * 1000).toLocaleString(locale, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

export function Meter({
  label,
  value,
  now,
}: {
  label: string;
  value: UsageWindow | null;
  now: number;
}) {
  const t = useCopy();
  const raw = currentPercent(value, now);
  // Rounded once, and this one figure is used for the label, every ARIA
  // value and the bar, so no channel can disagree with another: rounding
  // some and not others made 99.5 read "100%" beside `valuenow` 99.5.
  const percent = raw == null ? null : Math.round(raw);
  const passed = value != null && resetPassed(value, now);
  return (
    <div className="meter">
      <div className="meter-heading">
        <span>{label}</span>
        <strong>
          {percent != null ? `${percent}%` : passed ? t.resetPassed : "—"}
        </strong>
      </div>
      <div
        className="meter-track"
        role="meter"
        aria-label={label}
        // The label's own figure. The range stretches to contain it, below 0
        // as above 100, so `valuenow` always sits inside it as the spec
        // requires. The trade-off: 120/120 has the same ratio as 100/100 for
        // anything that computes one; what is announced is `valuetext`.
        // Whether readings should be clamped at all is a question for every
        // surface, not for this widget.
        aria-valuenow={percent ?? undefined}
        aria-valuemin={Math.min(0, percent ?? 0)}
        aria-valuemax={Math.max(100, percent ?? 0)}
        aria-valuetext={percent == null ? t.meterUnavailable : `${percent}%`}
      >
        <div
          className={percent != null && percent >= 90 ? "hot" : ""}
          style={{ width: `${clamp(percent ?? 0)}%` }}
        />
      </div>
      <span className="reset">
        {value?.resets_at != null
          ? t.resetsAt(when(t.locale, value.resets_at), passed)
          : t.resetUnknown}
      </span>
    </div>
  );
}

/**
 * A modal that owns its own pending and error state. Without `onConfirm` it
 * is informational and closes with a single button.
 */
export function Dialog({
  title,
  children,
  confirmLabel,
  onConfirm,
  onClose,
}: {
  title: string;
  children: ReactNode;
  confirmLabel?: string;
  onConfirm?: () => Promise<void>;
  onClose: () => void;
}) {
  const t = useCopy();
  const ref = useRef<HTMLDialogElement>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<unknown>(null);
  useEffect(() => ref.current?.showModal(), []);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (!onConfirm) return onClose();
    setPending(true);
    setError(null);
    try {
      await onConfirm();
      onClose();
    } catch (failure) {
      setError(failure);
      setPending(false);
    }
  };
  return (
    <dialog
      ref={ref}
      onCancel={(e) => {
        e.preventDefault();
        if (!pending) onClose();
      }}
    >
      <form onSubmit={submit}>
        <span className="eyebrow">{t.dialogEyebrow}</span>
        <h2>{title}</h2>
        <div className="dialog-body">{children}</div>
        {error != null && (
          <p className="dialog-error" role="alert">
            {describeError(t, error)}
          </p>
        )}
        <div className="dialog-actions">
          {onConfirm && (
            <button type="button" onClick={onClose} disabled={pending}>
              {t.cancel}
            </button>
          )}
          <button className="primary" type="submit" disabled={pending}>
            {pending ? t.working : (confirmLabel ?? t.done)}
          </button>
        </div>
      </form>
    </dialog>
  );
}

export function Empty({
  icon,
  title,
  children,
}: {
  icon?: keyof typeof paths;
  title?: string;
  children?: ReactNode;
}) {
  return (
    <div className="empty">
      {icon && <Icon name={icon} />}
      {title && <h3>{title}</h3>}
      {children && <p>{children}</p>}
    </div>
  );
}

/** A page-level failure with the one action that can fix it. */
export function Failure({ error, retry }: { error: unknown; retry: () => void }) {
  const t = useCopy();
  return (
    <div className="banner error" role="alert">
      <strong>{t.needsAttention}</strong>
      <span>{describeError(t, error)}</span>
      <button onClick={retry}>{t.retry}</button>
    </div>
  );
}
