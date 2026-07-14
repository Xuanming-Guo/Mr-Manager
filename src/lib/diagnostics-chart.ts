interface RecordingTimelineSample {
  collectedAtMs: number;
  system: {
    cpuTotalPercent: number;
    memory: { usedBytes: number; totalBytes: number };
  };
}

export function buildRecordingTimelineData(samples: RecordingTimelineSample[]) {
  const visibleSamples = samples.slice(-120);
  return {
    timestampsMs: visibleSamples.map((sample) => sample.collectedAtMs),
    cpuPercent: visibleSamples.map((sample) => clampPercent(sample.system.cpuTotalPercent)),
    ramPercent: visibleSamples.map((sample) => {
      const { usedBytes, totalBytes } = sample.system.memory;
      return clampPercent(totalBytes > 0 ? (usedBytes / totalBytes) * 100 : 0);
    }),
  };
}

function clampPercent(value: number) {
  return Number.isFinite(value) ? Math.min(100, Math.max(0, value)) : 0;
}
