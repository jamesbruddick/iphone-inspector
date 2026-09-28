import { PhoneArt } from '@/components/PhoneArt';
import { useFinishChoice } from '@/lib/finish-choice';
import { finishKey, readingFor } from '@/lib/readings';

interface ThumbDevice {
  udid: string;
  model: string | null;
  serial: string | null;
  /** The color the phone itself reported, if known. */
  colorHex?: string | null;
}

/** A phone's illustration, in the color it reported or the one picked for it. */
export function DeviceThumb({ device, className }: { device: ThumbDevice; className: string }) {
  const picked = useFinishChoice(device.model ? finishKey(device.udid, device.serial) : null, device.model);
  const reported = device.colorHex ?? readingFor(device.udid)?.colorHex ?? null;
  return <PhoneArt model={device.model ?? ''} hex={reported ?? picked?.hex ?? null} className={className} />;
}
