/** Mirrors the Rust `model` module. Kept hand-written: the surface is small and
 * a codegen step would cost more than it saves. */

export interface Item {
  id: string;
  text: string;
  done: boolean;
  createdAt: number;
  schedule?: Schedule;
}

export type Frequency = 'weekly' | 'monthly';
export interface Schedule {
  dueAt: number | null;
  remindBeforeMinutes: number | null;
  notifiedFor: number | null;
  repeat: { frequency: Frequency; resetAnchor: number; dueAnchor: number | null; occurrence: number } | null;
}
export interface ScheduleInput {
  dueAt: number | null;
  remindBeforeMinutes: number | null;
  frequency: Frequency | null;
  nextResetAt: number | null;
}
export interface Badge { count: number; urgent: number }

export interface Card {
  id: string;
  title: string;
  items: Item[];
  accent: number;
  createdAt: number;
  updatedAt: number;
}

export interface Skill {
  id: string;
  name: string;
  content: string;
  createdAt: number;
  updatedAt: number;
}

export interface Point {
  x: number;
  y: number;
}

export interface Workspace {
  version: number;
  cards: Card[];
  activeCardId: string | null;
  skills: Skill[];
  pinned: boolean;
  ballPosition: Point | null;
}
