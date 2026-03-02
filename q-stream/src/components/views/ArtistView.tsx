import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { useStore } from "../../store";
import * as api from "../../api";
import type { ArtistEnrichment } from "../../types";
import AlbumCard from "../cards/AlbumCard";

export default function ArtistView() {
  const { artistDetail, setView, setViewParam, setAlbumDetail } = useStore();
  const [enrichment, setEnrichment] = useState<ArtistEnrichment | null>(null);

  useEffect(() => {
    if (!artistDetail) return;
    setEnrichment(null);
    api.getArtistEnrichment(artistDetail.name)
      .then(setEnrichment)
      .catch(console.error);
  }, [artistDetail?.id]);

  if (!artistDetail) {
    return (
      <div className="flex items-center justify-center h-full text-qs-text-dim">
        No artist selected
      </div>
    );
  }

  const artist = artistDetail;
  const imageUrl = artist.image?.large;
  const genres = enrichment?.genres ?? [];
  const bio = artist.biography?.content ?? enrichment?.bio ?? null;

  return (
    <div className="p-8">
      {/* Header */}
      <div className="flex items-end gap-6 mb-6">
        <motion.div
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          className="w-48 h-48 rounded-full overflow-hidden shadow-2xl flex-shrink-0"
        >
          {imageUrl ? (
            <img src={imageUrl} alt={artist.name} className="w-full h-full object-cover" />
          ) : (
            <div className="w-full h-full bg-qs-surface flex items-center justify-center text-5xl">
              🎤
            </div>
          )}
        </motion.div>

        <div className="min-w-0">
          <p className="text-xs text-qs-text-dim uppercase tracking-wider mb-1">Artiste</p>
          <h1 className="text-5xl font-bold text-white mb-3">{artist.name}</h1>

          {/* Genre tags from MusicBrainz */}
          {genres.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {genres.map((g) => (
                <span
                  key={g}
                  className="px-2 py-0.5 text-xs rounded-full bg-white/10 text-qs-text-dim hover:bg-white/15 transition"
                >
                  {g}
                </span>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Biography — Qobuz first, Wikipedia as fallback */}
      {bio && (
        <div className="mb-8 max-w-2xl">
          <div className="flex items-center gap-2 mb-2">
            <h3 className="text-lg font-semibold text-white">À propos</h3>
            {!artist.biography?.content && enrichment?.bio && (
              <span className="text-[10px] px-1.5 py-0.5 rounded bg-white/5 text-qs-text-dim">
                via Wikipedia
              </span>
            )}
          </div>
          <p
            className="text-sm text-qs-text-dim leading-relaxed line-clamp-5"
            dangerouslySetInnerHTML={{ __html: bio }}
          />
        </div>
      )}

      {/* Albums */}
      {artist.albums?.items?.length ? (
        <section>
          <h3 className="text-lg font-semibold text-white mb-4">Discographie</h3>
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
            {artist.albums.items.map((album) => (
              <AlbumCard
                key={album.id}
                album={album}
                onClick={async () => {
                  const a = await api.getAlbum(album.id);
                  setAlbumDetail(a);
                  setViewParam(album.id);
                  setView("album");
                }}
              />
            ))}
          </div>
        </section>
      ) : null}

      {/* Similar Artists */}
      {artist.similar_artists?.length ? (
        <section className="mt-8">
          <h3 className="text-lg font-semibold text-white mb-4">Artistes similaires</h3>
          <div className="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
            {artist.similar_artists.map((sa) => (
              <button
                key={sa.id}
                onClick={async () => {
                  const a = await api.getArtist(sa.id);
                  useStore.getState().setArtistDetail(a);
                  setViewParam(sa.id.toString());
                }}
                className="flex flex-col items-center gap-2 p-3 rounded-xl hover:bg-white/5 transition"
              >
                <div className="w-20 h-20 rounded-full overflow-hidden bg-qs-surface">
                  {sa.image?.large ? (
                    <img src={sa.image.large} alt={sa.name} className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-2xl">🎤</div>
                  )}
                </div>
                <p className="text-xs font-medium text-white truncate w-full text-center">{sa.name}</p>
              </button>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
