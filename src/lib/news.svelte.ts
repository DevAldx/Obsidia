import { api } from "./api";
import type { NewsFeed, NewsItem } from "./types";

/**
 * Noticias de Minecraft compartidas por Inicio y Descubrir. Rust las guarda
 * en caché (memoria + disco), así que pedirlas varias veces es barato.
 */
class NewsState {
  feed = $state<NewsFeed | null>(null);
  loading = $state(false);
  error = $state("");

  articles = $derived<NewsItem[]>(this.feed?.items.filter((i) => i.kind === "news") ?? []);
  versions = $derived<NewsItem[]>(this.feed?.items.filter((i) => i.kind !== "news") ?? []);

  async load(force = false) {
    if (this.loading) return;
    if (this.feed && !force) return;
    this.loading = true;
    this.error = "";
    try {
      this.feed = await api.fetchNews(force);
    } catch (e) {
      this.error = e instanceof Error ? e.message : String(e);
    } finally {
      this.loading = false;
    }
  }
}

export const news = new NewsState();
