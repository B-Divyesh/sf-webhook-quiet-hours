import { describe, expect, it } from 'vitest';
import { escapeHtml, formatUtcOffset, relativeTime } from './utils';

describe('display utilities', () => {
  it('escapes untrusted webhook labels', () => expect(escapeHtml('<img onerror=alert(1)>')).not.toContain('<img'));
  it('formats whole-hour offsets', () => expect(formatUtcOffset(330)).toBe('UTC+05:30'));
  it('uses readable relative time', () => expect(relativeTime('2026-08-27T11:59:00Z', Date.parse('2026-08-27T12:00:00Z'))).toBe('1 minute ago'));
});
