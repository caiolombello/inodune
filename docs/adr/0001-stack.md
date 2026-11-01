# ADR 0001 — Rust + Tauri 2 + Svelte 5

Status: aceito para Fase 0, 31/10/2026. Contexto: desktop Linux local, sem privilégios, autoridade FS única e UI premium.

Decisão: Rust workspace independente do renderer, Tauri2 sem plugins/capacidades FS/shell, Svelte5/TypeScript/Vite, CSS tokens locais, lockfiles. A CLI compartilha core. Browser serve somente para prototipar/verificar renderer; não substitui aceite nativo. JSON sintético único testa contrato Rust/TS; nenhum backend de limpeza registrado.

Alternativas: Electron adiciona runtime/memória; GTK nativo exigiria UI separada do TS exigido. Consequência: WebKitGTK/GTK3 são dependências reais de compilação e runtime, documentadas por distro. Sem sudo durante esta tarefa. API de limpeza e traversal ficam adiadas até gates de segurança.
