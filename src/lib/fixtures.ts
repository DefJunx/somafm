import type { Station } from "./api";

export function station(id: string, genres: string[] = [], over: Partial<Station> = {}): Station {
  return {
    id, title: `Station ${id}`, description: "", dj: "", genres, listeners: 0,
    lastPlaying: "", image: `https://img.example/${id}.png`,
    largeImage: `https://img.example/${id}-l.png`, xlImage: `https://img.example/${id}-xl.png`,
    ...over,
  };
}
