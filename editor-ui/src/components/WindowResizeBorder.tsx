import { sendToRust } from '../ipc/bridge';

const BORDER = 5;

const zones: { direction: string; style: React.CSSProperties; cursor: string }[] = [
  // Edges
  { direction: 'n',  cursor: 'ns-resize',   style: { top: 0, left: BORDER, right: BORDER, height: BORDER } },
  { direction: 's',  cursor: 'ns-resize',   style: { bottom: 0, left: BORDER, right: BORDER, height: BORDER } },
  { direction: 'w',  cursor: 'ew-resize',   style: { left: 0, top: BORDER, bottom: BORDER, width: BORDER } },
  { direction: 'e',  cursor: 'ew-resize',   style: { right: 0, top: BORDER, bottom: BORDER, width: BORDER } },
  // Corners
  { direction: 'nw', cursor: 'nwse-resize', style: { top: 0, left: 0, width: BORDER, height: BORDER } },
  { direction: 'ne', cursor: 'nesw-resize', style: { top: 0, right: 0, width: BORDER, height: BORDER } },
  { direction: 'sw', cursor: 'nesw-resize', style: { bottom: 0, left: 0, width: BORDER, height: BORDER } },
  { direction: 'se', cursor: 'nwse-resize', style: { bottom: 0, right: 0, width: BORDER, height: BORDER } },
];

export function WindowResizeBorder() {
  return (
    <>
      {zones.map((z) => (
        <div
          key={z.direction}
          onMouseDown={(e) => {
            e.preventDefault();
            sendToRust({ type: 'window_resize_start', direction: z.direction });
          }}
          style={{
            position: 'fixed',
            zIndex: 99999,
            cursor: z.cursor,
            ...z.style,
          }}
        />
      ))}
    </>
  );
}
