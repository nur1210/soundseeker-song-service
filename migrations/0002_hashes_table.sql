CREATE TABLE public.hashes
(
    hid    integer NOT NULL,
    hash   bigint,
    "time" integer,
    sid    integer
);

CREATE SEQUENCE public.hashes_hid_seq AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE Cache 1;

ALTER SEQUENCE public.hashes_hid_seq OWNED BY public.hashes.hid;

ALTER TABLE ONLY public.hashes
    ALTER COLUMN hid SET DEFAULT nextval('public.hashes_hid_seq'::regclass);

ALTER TABLE ONLY public.hashes
    ADD CONSTRAINT hashes_pkey PRIMARY KEY (hid);

CREATE UNIQUE INDEX hashes_hash_time_sid_idx ON public.hashes USING btree (hash, "time", sid);

CREATE UNIQUE INDEX hashes_time_sid_idx ON public.hashes USING btree ("time", sid);

ALTER TABLE ONLY public.hashes
    ADD CONSTRAINT sid FOREIGN KEY (sid) REFERENCES public.songs (sid) ON
        UPDATE CASCADE
        ON
            DELETE CASCADE;