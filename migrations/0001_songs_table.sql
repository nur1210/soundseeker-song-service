CREATE TABLE public.songs
(
    sid  integer NOT NULL,
    song text
);

CREATE SEQUENCE public.songs_sid_seq AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE Cache 1;

ALTER SEQUENCE public.songs_sid_seq OWNED BY songs.sid;

ALTER TABLE ONLY public.songs
    ALTER COLUMN sid SET DEFAULT nextval('public.songs_sid_seq'::regclass);

ALTER TABLE ONLY public.songs
    ADD CONSTRAINT songs_pkey PRIMARY KEY (sid);

ALTER TABLE ONLY public.songs
    ADD CONSTRAINT songs_song_key UNIQUE (song);