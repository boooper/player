const AUDIODB_API_URL = 'https://www.theaudiodb.com/api/v1/json/2';

export type AudioDbArtist = {
  name: string;
  thumb: string;
  fanart: string;
  banner: string;
  biography: string;
  genre: string;
  country: string;
  formedYear: string;
};

export async function fetchAudioDbArtist(artistName: string): Promise<AudioDbArtist | null> {
  if (!artistName.trim()) return null;
  try {
    const url = new URL(`${AUDIODB_API_URL}/search.php`);
    url.searchParams.set('s', artistName);
    const response = await fetch(url);
    if (!response.ok) return null;
    const json = await response.json();
    const a = json?.artists?.[0];
    if (!a) return null;
    return {
      name: String(a.strArtist ?? ''),
      thumb: String(a.strArtistThumb ?? ''),
      fanart: String(a.strArtistFanart ?? ''),
      banner: String(a.strArtistBanner ?? ''),
      biography: String(a.strBiographyEN ?? ''),
      genre: String(a.strGenre ?? ''),
      country: String(a.strCountry ?? ''),
      formedYear: String(a.intFormedYear ?? '')
    };
  } catch {
    return null;
  }
}

export async function fetchAudioDbArtistPhoto(artistName: string): Promise<string> {
  const artist = await fetchAudioDbArtist(artistName);
  if (!artist) return '';
  return artist.thumb || artist.fanart || artist.banner || '';
}
