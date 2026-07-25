import { useId } from "react";
import type {
  InputHTMLAttributes,
  ReactNode,
  SelectHTMLAttributes,
  TextareaHTMLAttributes,
} from "react";

interface FieldShellProps {
  label: string;
  hint?: string;
  error?: string;
  children: (ids: { inputId: string; describedBy: string | undefined }) => ReactNode;
}

function FieldShell({ label, hint, error, children }: FieldShellProps) {
  const inputId = useId();
  const hintId = `${inputId}-hint`;
  const errorId = `${inputId}-error`;
  const describedBy = [hint ? hintId : null, error ? errorId : null].filter(Boolean).join(" ");

  return (
    <div className={`field${error ? " field--invalid" : ""}`}>
      <label className="field__label" htmlFor={inputId}>
        {label}
      </label>
      {children({ inputId, describedBy: describedBy || undefined })}
      {hint && !error && (
        <p className="field__hint" id={hintId}>
          {hint}
        </p>
      )}
      {error && (
        <p className="field__error" id={errorId}>
          {/* Icon plus text, so colour is never the only signal. */}
          <span aria-hidden="true">!</span> {error}
        </p>
      )}
    </div>
  );
}

export interface TextFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "id"> {
  label: string;
  hint?: string;
  error?: string;
}

export function TextField({ label, hint, error, ...rest }: TextFieldProps) {
  return (
    <FieldShell label={label} hint={hint} error={error}>
      {({ inputId, describedBy }) => (
        <input
          id={inputId}
          className="input"
          aria-describedby={describedBy}
          aria-invalid={error ? true : undefined}
          {...rest}
        />
      )}
    </FieldShell>
  );
}

export interface TextAreaFieldProps
  extends Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, "id"> {
  label: string;
  hint?: string;
  error?: string;
  mono?: boolean;
}

export function TextAreaField({ label, hint, error, mono, ...rest }: TextAreaFieldProps) {
  return (
    <FieldShell label={label} hint={hint} error={error}>
      {({ inputId, describedBy }) => (
        <textarea
          id={inputId}
          className={`input input--textarea${mono ? " input--mono" : ""}`}
          aria-describedby={describedBy}
          aria-invalid={error ? true : undefined}
          spellCheck={mono ? false : undefined}
          {...rest}
        />
      )}
    </FieldShell>
  );
}

export interface SelectFieldProps extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "id"> {
  label: string;
  hint?: string;
  error?: string;
  options: { value: string; label: string }[];
}

export function SelectField({ label, hint, error, options, ...rest }: SelectFieldProps) {
  return (
    <FieldShell label={label} hint={hint} error={error}>
      {({ inputId, describedBy }) => (
        <select id={inputId} className="input input--select" aria-describedby={describedBy} {...rest}>
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      )}
    </FieldShell>
  );
}

export interface CheckboxFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "id"> {
  label: string;
  hint?: string;
}

export function CheckboxField({ label, hint, ...rest }: CheckboxFieldProps) {
  const inputId = useId();
  const hintId = `${inputId}-hint`;

  return (
    <div className="checkbox">
      <input
        id={inputId}
        type="checkbox"
        className="checkbox__input"
        aria-describedby={hint ? hintId : undefined}
        {...rest}
      />
      <div className="checkbox__text">
        <label htmlFor={inputId}>{label}</label>
        {hint && (
          <p className="field__hint" id={hintId}>
            {hint}
          </p>
        )}
      </div>
    </div>
  );
}
