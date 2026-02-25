import { motion } from "framer-motion";
import { useStore } from "../../store";
import * as api from "../../api";
import AlbumCard from "../cards/AlbumCard";

export default function ArtistView() {
  const { artistDetail, setView, setViewParam, setAlbumDetail } = useStore();

  if (!artistDetail) {
    return (
      <div className="flex items-center justify-center h-full text-qs-text-dim">
        No artist selected
      </div>
    );
  }

  const artist = artistDetail;
  const imageUrl = artist.image?.large;

  return (
    <div className="p-8">
      {/* Header */}
      <div className="flex items-end gap-6 mb-8">
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

        <div>
          <p className="text-xs text-qs-text-dim uppercase tracking-wider mb-1">Artist</p>
          <h1 className="text-5xl font-bold text-white mb-2">{artist.name}</h1>
        </div>
      </div>

      {/* Biography */}
      {artist.biography?.content && (
        <div className="mb-8 max-w-2xl">
          <h3 className="text-lg font-semibold text-white mb-2">About</h3>
          <p
            className="text-sm text-qs-text-dim leading-relaxed line-clamp-4"
            dangerouslySetInnerHTML={{ __html: artist.biography.content }}
          />
        </div>
      )}

      {/* Albums */}
      {artist.albums?.items?.length ? (
        <section>
          <h3 className="text-lg font-semibold text-white mb-4">Discography</h3>
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
          <h3 className="text-lg font-semibold text-white mb-4">Similar Artists</h3>
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
