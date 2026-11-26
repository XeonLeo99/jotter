import type { SunsetSettings } from './types';

const GEOCODE_URL = 'https://geocoding-api.open-meteo.com/v1/search';
const FORECAST_URL = 'https://api.open-meteo.com/v1/forecast';

export interface SunsetLocation {
  name: string;
  latitude: number;
  longitude: number;
  timezone: string;
}

export interface SunsetForecast {
  location: string;
  date: string;
  sunset: string;
  score: number;
  label: string;
  highCloud: number | null;
  midCloud: number | null;
  lowCloud: number | null;
  humidity: number | null;
  precipitation: number | null;
  visibilityKm: number | null;
}

interface GeocodeResponse {
  results?: Array<{
    name?: string;
    latitude?: number;
    longitude?: number;
    timezone?: string;
    admin1?: string;
    country?: string;
  }>;
}

interface ForecastResponse {
  daily?: { time?: string[]; sunset?: string[] };
  hourly?: {
    time?: string[];
    cloud_cover_high?: Array<number | null>;
    cloud_cover_mid?: Array<number | null>;
    cloud_cover_low?: Array<number | null>;
    relative_humidity_2m?: Array<number | null>;
    precipitation_probability?: Array<number | null>;
    visibility?: Array<number | null>;
  };
  timezone?: string;
}

function clamp(value: number, min = 0, max = 100): number {
  return Math.min(max, Math.max(min, value));
}

function numberAt(values: Array<number | null> | undefined, index: number): number | null {
  const value = values?.[index];
  return typeof value === 'number' && Number.isFinite(value) ? value : null;
}

function band(value: number | null, start: number, end: number): number {
  if (value == null) return 0.5;
  if (value <= start) return 0;
  if (value <= end) return 1;
  return clamp((100 - value) / (100 - end), 0, 1);
}

function localMinutes(value: string): number {
  const match = /T(\d{2}):(\d{2})/.exec(value);
  return match ? Number(match[1]) * 60 + Number(match[2]) : 0;
}

function machineDateKey(date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

function locationParts(timezone: string | null): Record<string, string> {
  if (!timezone) return {};
  try {
    const parts = new Intl.DateTimeFormat('en-US', {
      timeZone: timezone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      hour12: false,
      hourCycle: 'h23',
    }).formatToParts(new Date());
    return Object.fromEntries(parts.map((part) => [part.type, part.value]));
  } catch {
    return {};
  }
}

function locationDateKey(timezone: string | null): string {
  const values = locationParts(timezone);
  if (values.year && values.month && values.day) return `${values.year}-${values.month}-${values.day}`;
  return machineDateKey();
}

export function currentLocalHour(timezone: string | null): number {
  const hour = Number(locationParts(timezone).hour);
  return Number.isFinite(hour) ? hour : new Date().getHours();
}

export function todayDateKey(timezone: string | null = null): string {
  return timezone ? locationDateKey(timezone) : machineDateKey();
}

export async function geocodeLocation(query: string): Promise<SunsetLocation> {
  // Settings show a canonical “name · region · country” label after the first
  // lookup; geocoding the primary place name keeps subsequent saves editable.
  const name = query.split('·', 1)[0]?.trim() || query.trim();
  const params = new URLSearchParams({
    name,
    count: '1',
    language: 'zh',
    format: 'json',
  });
  const response = await fetch(`${GEOCODE_URL}?${params}`);
  if (!response.ok) throw new Error(`地点查询失败（HTTP ${response.status}）`);
  const data = (await response.json()) as GeocodeResponse;
  const result = data.results?.[0];
  if (
    !result ||
    typeof result.name !== 'string' ||
    typeof result.latitude !== 'number' ||
    typeof result.longitude !== 'number' ||
    typeof result.timezone !== 'string'
  ) {
    throw new Error('没有找到这个地点，请输入城市或区县名称');
  }
  const parts = [result.name, result.admin1, result.country].filter(
    (part): part is string => Boolean(part),
  );
  return {
    name: [...new Set(parts)].join(' · '),
    latitude: result.latitude,
    longitude: result.longitude,
    timezone: result.timezone,
  };
}

export async function fetchSunsetForecast(settings: SunsetSettings): Promise<SunsetForecast> {
  if (settings.latitude == null || settings.longitude == null || !settings.location) {
    throw new Error('请先设置晚霞地点');
  }
  const params = new URLSearchParams({
    latitude: settings.latitude.toFixed(5),
    longitude: settings.longitude.toFixed(5),
    daily: 'sunset',
    hourly:
      'cloud_cover,cloud_cover_low,cloud_cover_mid,cloud_cover_high,relative_humidity_2m,precipitation_probability,visibility',
    forecast_days: '2',
    timezone: 'auto',
  });
  const response = await fetch(`${FORECAST_URL}?${params}`);
  if (!response.ok) throw new Error(`晚霞数据获取失败（HTTP ${response.status}）`);
  const data = (await response.json()) as ForecastResponse;
  const dailyTimes = data.daily?.time ?? [];
  const sunsets = data.daily?.sunset ?? [];
  const targetDate = locationDateKey(settings.timezone);
  const dayIndex = Math.max(0, dailyTimes.findIndex((date) => date === targetDate));
  const sunset = sunsets[dayIndex] ?? sunsets[0];
  if (!sunset) throw new Error('天气服务暂时没有返回日落时间');

  const hourlyTimes = data.hourly?.time ?? [];
  const targetMinutes = localMinutes(sunset);
  let index = 0;
  let distance = Number.POSITIVE_INFINITY;
  hourlyTimes.forEach((time, i) => {
    if (!time.startsWith(sunset.slice(0, 10))) return;
    const next = Math.abs(localMinutes(time) - targetMinutes);
    if (next < distance) {
      distance = next;
      index = i;
    }
  });

  const highCloud = numberAt(data.hourly?.cloud_cover_high, index);
  const midCloud = numberAt(data.hourly?.cloud_cover_mid, index);
  const lowCloud = numberAt(data.hourly?.cloud_cover_low, index);
  const humidity = numberAt(data.hourly?.relative_humidity_2m, index);
  const precipitation = numberAt(data.hourly?.precipitation_probability, index);
  const visibility = numberAt(data.hourly?.visibility, index);
  const visibilityKm = visibility == null ? null : visibility / 1000;

  // This is a transparent forecast estimate: high/mid clouds often catch the
  // warm light, while low cloud, rain and haze hide the horizon.
  const base =
    band(highCloud, 20, 75) * 45 +
    band(midCloud, 10, 65) * 25 +
    (lowCloud == null ? 0.5 : (100 - clamp(lowCloud)) / 100) * 20 +
    (visibilityKm == null ? 0.5 : clamp(visibilityKm / 20, 0, 1)) * 10;
  const rainFactor = 1 - clamp(precipitation ?? 0) / 160;
  const score = Math.round(clamp(base * rainFactor));
  const label = score < 30 ? '暗淡' : score < 60 ? '一般' : '较强';

  return {
    location: settings.location,
    date: sunset.slice(0, 10),
    sunset,
    score,
    label,
    highCloud,
    midCloud,
    lowCloud,
    humidity,
    precipitation,
    visibilityKm,
  };
}

export function formatSunsetTime(value: string): string {
  const match = /T(\d{2}:\d{2})/.exec(value);
  return match?.[1] ?? value;
}

export function formatSunsetDetails(forecast: SunsetForecast): string {
  const cloud = [
    forecast.highCloud == null ? null : `高云 ${Math.round(forecast.highCloud)}%`,
    forecast.midCloud == null ? null : `中云 ${Math.round(forecast.midCloud)}%`,
    forecast.lowCloud == null ? null : `低云 ${Math.round(forecast.lowCloud)}%`,
  ]
    .filter((value): value is string => value != null)
    .join('，');
  const extras = [
    cloud,
    forecast.visibilityKm == null ? null : `能见度 ${forecast.visibilityKm.toFixed(1)} km`,
    forecast.precipitation == null ? null : `降水概率 ${Math.round(forecast.precipitation)}%`,
  ].filter((value): value is string => Boolean(value));
  return `日落 ${formatSunsetTime(forecast.sunset)}，预测晚霞强度：${forecast.label}（${forecast.score}/100）。${extras.join('；')}。数据源：Open-Meteo。`;
}
