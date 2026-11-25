export function sortMarks(marks) {
  return marks
    .filter((mark) => mark.value)
    .sort((a, b) => a.time - b.time);
}

export function pollyMarksFromJsonLines(text) {
  return sortMarks(
    text
      .split('\n')
      .map((line) => (line ? JSON.parse(line) : null))
      .filter(Boolean)
      .map((mark) => ({ time: mark.time, value: mark.value }))
  );
}

export function googleMarksFromTimepoints(timepoints) {
  return sortMarks(
    (timepoints || []).map((point) => ({
      // JSON may omit a zero time.
      time: Math.round(Number(point.timeSeconds ?? 0) * 1000),
      value: point.markName,
    }))
  );
}

export function azureMarksFromEvents(events) {
  return sortMarks(
    (events || []).map((event) => ({
      time: Math.round(Number(event.audioOffset) / 10000),
      value: event.text,
    }))
  );
}
