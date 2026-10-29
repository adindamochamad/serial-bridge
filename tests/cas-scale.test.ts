import { describe, it, expect, beforeEach } from 'vitest';
import {
  isCh340Adapter,
  isCasDevice,
  resetCasWarmup,
} from '../src/lib/cas-scale.js';
import type { DiscoveredDevice } from '../src/lib/types.js';

function sampleDevice(patch: Partial<DiscoveredDevice> = {}): DiscoveredDevice {
  return {
    port: 'COM3',
    device_class: 'unknown',
    label: 'Unknown',
    manufacturer: null,
    product: null,
    vid: null,
    pid: null,
    serial_number: null,
    suggested_baud: 9600,
    ...patch,
  };
}

describe('isCh340Adapter', () => {
  it('recognizes device_class ch340', () => {
    expect(isCh340Adapter(sampleDevice({ device_class: 'ch340' }))).toBe(true);
  });

  it('recognizes WCH VID 0x1A86', () => {
    expect(isCh340Adapter(sampleDevice({ vid: 0x1a86 }))).toBe(true);
  });
});

describe('isCasDevice', () => {
  beforeEach(() => resetCasWarmup());

  it('treats COM3 CH340 as a CAS scale', () => {
    expect(
      isCasDevice(
        sampleDevice({
          port: 'COM3',
          device_class: 'ch340',
          label: 'CH340/CH341',
          vid: 0x1a86,
        })
      )
    ).toBe(true);
  });

  it('does not treat a generic FTDI port as CAS', () => {
    expect(
      isCasDevice(
        sampleDevice({
          device_class: 'ftdi_adapter',
          manufacturer: 'FTDI',
          product: 'FT232R',
        })
      )
    ).toBe(false);
  });
});
