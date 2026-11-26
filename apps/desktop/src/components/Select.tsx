import {realPointerMove} from '../lib/controller';
import { useEffect, useId, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown } from 'lucide-react';
import type { InputAction } from '../lib/types';

export function Select<T extends string>({ label, value, options, onChange, disabled = false }: {
  label: string; value: T; options: { value: T; label: string }[]; onChange: (value: T) => void; disabled?: boolean;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const popup = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [rect, setRect] = useState({ left: 0, top: 0, width: 240, maxHeight: 300 });
  const close = () => { setOpen(false); trigger.current?.focus(); };
  const show = () => {
    if (disabled) return;
    const box = trigger.current!.getBoundingClientRect();
    const room = window.innerHeight - box.bottom - 20;
    const height = Math.min(320, options.length * 46 + 12);
    const above = room < Math.min(height, 180) && box.top > room;
    setRect({ left: Math.max(12, Math.min(box.left, window.innerWidth - Math.max(box.width, 240) - 12)),
      top: above ? Math.max(12, box.top - height - 8) : box.bottom + 8,
      width: Math.min(Math.max(box.width, 240), window.innerWidth - 24), maxHeight: above ? Math.min(height, box.top - 20) : Math.min(height, room) });
    setActive(Math.max(0, options.findIndex(o => o.value === value)));
    setOpen(true);
  };
  const action = (a: InputAction) => {
    if (!open) { if (a === 'CONFIRM') { show(); return true; } return false; }
    if (a === 'UP' || a === 'DOWN') setActive(i => Math.max(0, Math.min(options.length - 1, i + (a === 'UP' ? -1 : 1))));
    if (a === 'CONFIRM') { if (options[active]) onChange(options[active].value); close(); }
    if (a === 'BACK') close();
    return true;
  };
  useEffect(() => {
    const onPad = (event: Event) => {
      if ((open || document.activeElement === trigger.current) && action((event as CustomEvent<InputAction>).detail)) event.preventDefault();
    };
    window.addEventListener('deck-control', onPad);
    return () => window.removeEventListener('deck-control', onPad);
  });
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => { if (!trigger.current?.contains(event.target as Node) && !popup.current?.contains(event.target as Node)) setOpen(false); };
    const reposition = () => setOpen(false);
    document.addEventListener('pointerdown', outside);
    window.addEventListener('resize', reposition);
    return () => { document.removeEventListener('pointerdown', outside); window.removeEventListener('resize', reposition); };
  }, [open]);
  useEffect(() => { popup.current?.querySelector('[data-active="true"]')?.scrollIntoView({ block: 'nearest' }); }, [active, open]);
  return <span className="deck-select" data-input-capture>
    <button ref={trigger} className="deck-select-trigger" type="button" role="combobox" aria-label={label}
      aria-controls={id} aria-expanded={open} aria-haspopup="listbox" aria-activedescendant={open ? `${id}-${active}` : undefined}
      disabled={disabled} onClick={() => open ? close() : show()}
      onKeyDown={e => {
        const map: Record<string, InputAction> = { ArrowDown: 'DOWN', ArrowUp: 'UP', Enter: 'CONFIRM', ' ': 'CONFIRM', Escape: 'BACK' };
        if (e.key === 'Tab') { setOpen(false); return; }
        if (map[e.key]) { e.preventDefault(); e.stopPropagation(); if (!open && ['ArrowDown', 'ArrowUp'].includes(e.key)) show(); else action(map[e.key]); }
        else if (open && e.key.length === 1) { const i = options.findIndex(o => o.label.toLowerCase().startsWith(e.key.toLowerCase())); if (i >= 0) setActive(i); }
      }}><span>{options.find(o => o.value === value)?.label || label}</span><ChevronDown size={16} className={open ? 'select-up' : ''}/></button>
    {open && createPortal(<div ref={popup} id={id} role="listbox" aria-label={label} className="deck-options" style={rect} data-input-capture>
      {options.map((o, i) => <div id={`${id}-${i}`} key={o.value} role="option" aria-selected={o.value === value} data-active={i === active}
        onPointerMove={event => {if(realPointerMove(event))setActive(i);}} onMouseDown={e => e.preventDefault()} onClick={() => { onChange(o.value); close(); }}>
        <span>{o.label}</span>{o.value === value && <Check size={16}/>}</div>)}
    </div>, document.body)}
  </span>;
}
