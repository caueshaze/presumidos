-- Partidas (placeholder) da Copa do Mundo 2026

INSERT INTO matches (id, home_team, away_team, kickoff, group_name) VALUES
    (lower(hex(randomblob(16))), 'Brasil', 'Argentina', '2026-06-12T18:00:00Z', 'A'),
    (lower(hex(randomblob(16))), 'Alemanha', 'França', '2026-06-13T15:00:00Z', 'B'),
    (lower(hex(randomblob(16))), 'Espanha', 'Itália', '2026-06-13T18:00:00Z', 'A'),
    (lower(hex(randomblob(16))), 'Portugal', 'Holanda', '2026-06-14T15:00:00Z', 'B'),
    (lower(hex(randomblob(16))), 'Inglaterra', 'Bélgica', '2026-06-14T18:00:00Z', 'C'),
    (lower(hex(randomblob(16))), 'Estados Unidos', 'México', '2026-06-15T15:00:00Z', 'C'),
    (lower(hex(randomblob(16))), 'Japão', 'Coreia do Sul', '2026-06-15T18:00:00Z', 'D'),
    (lower(hex(randomblob(16))), 'Uruguai', 'Croácia', '2026-06-16T15:00:00Z', 'D'),
    (lower(hex(randomblob(16))), 'Marrocos', 'Senegal', '2026-06-16T18:00:00Z', 'A'),
    (lower(hex(randomblob(16))), 'Canadá', 'Suíça', '2026-06-17T15:00:00Z', 'B');
