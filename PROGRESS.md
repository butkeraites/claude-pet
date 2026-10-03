# Andamento

Uma linha por tarefa concluída: data, tarefa, o quê, commit.

| Data | Tarefa | O quê | Commit |
|---|---|---|---|
| 2026-10-02 | T0.1 | Repositório criado: plano aprovado, decisões 0001–0015, CLAUDE.md, README, licença e créditos | 4e38255 |
| 2026-10-02 | T0.2 | Pesquisa de origem (hooks, motor, Hyprland, Docker, arte, inspirações, desenho e revisões) em `docs/pesquisa/` | a975ad6 |
| 2026-10-02 | T0.3 | Workspace Rust (edition 2024, toolchain stable) com `pet-core`, `claude-pet` e `xtask` | 9932081 |
| 2026-10-02 | T0.4 | Daemon: config com precedência e origem, `/saude`, `/v1/estado`, checagem de Host e `X-Pet`, vigia de 60 s, SIGTERM, subcomando `saude`; 27 testes | 9932081 |
| 2026-10-02 | T0.5 | Docker (alpine fixada por digest, imagem de 4,3 MB, RSS < 1 MiB, healthy) com bind de `/run/user` em rslave, compose de dev e `bin/pet` (`subir`, `parar`, `logs`, `estado`, `reconstruir`, `dev`, `verificar`) | 74084d3 |
| 2026-10-02 | T0.6 | Repositório privado `butkeraites/claude-pet` no GitHub, `main` publicada | — |
| 2026-10-02 | T1.1 | Descoberta do compositor (`hyprland.lock`, assinatura mais nova, `connect()` de prova no `.socket2.sock` e no `wayland-N`, symlink curto para caminho longo), laço calloop com batimento de 5 s, sinais por pipe e reconexão com backoff só na mesma assinatura; testado ao vivo com um proxy derrubando a conexão | — |
| 2026-10-02 | T1.2 | Camada OVERLAY `claude-pet` criada com output NULL (4 âncoras, `exclusive_zone -1`, teclado NONE), mapeamento inicial 1x1 transparente com viewport e região de input vazia, escala por `wp_fractional_scale_v1` com reservas (modo do monitor em 200 ms, primeiro monitor utilizável em 500 ms) e `closed` recriando em 250 ms; ao vivo: eDP-1 1280x800, `preferred_scale` 180/120, buffer 1920x1200, nível 3 no `hyprctl layers` | — |
| 2026-10-02 | T1.3 | Core puro: `skin` (skin.json + folha json-array do Aseprite, PNG para RGBA e BGRA pré-multiplicado, validação com avisos), `geometria` (D por monitor, posição padrão, região lógica por fora), `raster` (blocos D×D, espelho, recorte, "over", dano em lista com ladrilhos de 64×64), `animador` (tags com as 4 direções, repouso com pose e rajadas < 1 commit/s), `cena` (dano entre cenas e quadro esperado); `cargo xtask skin-teste` gera `skins/_teste` (56 quadros, 19 tags); quadros dourados por SHA-256 | — |
