//! Configuração com precedência visível.
//!
//! Ordem (a última vence): padrões do código < arquivo `bichinho.toml` <
//! variáveis `PET_<SECAO>_<CHAVE>` < comandos dados em tempo de execução
//! (gravados em `/state`, a partir do M3). Cada chave efetiva guarda de onde
//! veio; o `/v1/estado` mostra isso para o Renan saber por que o pet está
//! fazendo o que está fazendo.
//!
//! Valor inválido nunca derruba o pet: vira um aviso e a camada anterior
//! continua valendo.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::geometria::Tamanho;

/// De onde veio o valor efetivo de uma chave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origem {
    Padrao,
    Arquivo,
    Ambiente,
    Comando,
}

/// Valor de uma chave.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Valor {
    Texto(String),
    Lista(Vec<String>),
    /// Os pesos e os limites da pontuação (decisão 0074).
    Numero(f64),
}

#[derive(Debug, Clone, Copy)]
enum Tipo {
    /// Identificador curto: `[a-z0-9_-]{1,40}` (nome de skin, por exemplo).
    Id,
    /// Uma das opções da lista.
    Opcao(&'static [&'static str]),
    /// Lista de até [`MAX_ITENS`] identificadores, sem repetição. No arquivo,
    /// um array TOML; no ambiente, separados por vírgula (vazio = lista
    /// vazia).
    ListaDeIds,
    /// Um número (inteiro ou decimal) na faixa; no ambiente, com ponto.
    Numero { min: f64, max: f64 },
}

/// Maior lista aceita numa chave.
const MAX_ITENS: usize = 16;

struct Chave {
    caminho: &'static str,
    tipo: Tipo,
    /// Para listas, os itens separados por vírgula.
    padrao: &'static str,
}

const MODOS_CELEBRACAO: &[&str] = &["proporcional", "sempre_grande", "discreta", "desligada"];

const CHAVES: &[Chave] = &[
    Chave {
        caminho: "aparencia.skin",
        tipo: Tipo::Id,
        padrao: "zeca",
    },
    // O tamanho do pet na tela (decisão 0042): muda só o D, nunca a skin.
    Chave {
        caminho: "aparencia.tamanho",
        tipo: Tipo::Opcao(Tamanho::NOMES),
        padrao: "normal",
    },
    Chave {
        caminho: "celebracao.modo",
        tipo: Tipo::Opcao(MODOS_CELEBRACAO),
        padrao: "proporcional",
    },
    // De onde vêm as sessões que contam (`$CLAUDE_CODE_ENTRYPOINT`): só o
    // terminal por padrão; `claude -p`, SDK e IDE ficam de fora (decisão
    // 0020).
    Chave {
        caminho: "sessoes.origens",
        tipo: Tipo::ListaDeIds,
        padrao: "cli",
    },
    // O T3 no máximo a cada tanto (decisão 0074).
    Chave {
        caminho: "celebracao.intervalo_t3_min",
        tipo: Tipo::Numero {
            min: 0.0,
            max: 1440.0,
        },
        padrao: "10",
    },
    // A pontuação de cada turno pelo trabalho (decisões 0003 e 0074).
    Chave {
        caminho: "pontuacao.por_minuto",
        tipo: PESO,
        padrao: "1.0",
    },
    Chave {
        caminho: "pontuacao.por_ferramenta_de_trabalho",
        tipo: PESO,
        padrao: "0.15",
    },
    Chave {
        caminho: "pontuacao.por_outra_ferramenta",
        tipo: PESO,
        padrao: "0.05",
    },
    Chave {
        caminho: "pontuacao.por_arquivo",
        tipo: PESO,
        padrao: "0.5",
    },
    Chave {
        caminho: "pontuacao.por_subagente",
        tipo: PESO,
        padrao: "1.0",
    },
    Chave {
        caminho: "pontuacao.teto",
        tipo: LIMITE,
        padrao: "20",
    },
    Chave {
        caminho: "pontuacao.t2",
        tipo: LIMITE,
        padrao: "4",
    },
    Chave {
        caminho: "pontuacao.t3",
        tipo: LIMITE,
        padrao: "12",
    },
];

/// Um peso da pontuação.
const PESO: Tipo = Tipo::Numero {
    min: 0.0,
    max: 100.0,
};
/// Um limite da pontuação (o teto, o começo do T2 e o do T3).
const LIMITE: Tipo = Tipo::Numero {
    min: 0.0,
    max: 1000.0,
};

/// Valor efetivo de uma chave e a camada de onde ele veio.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entrada {
    pub valor: Valor,
    pub origem: Origem,
}

/// Configuração depois de aplicar todas as camadas.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ConfigEfetiva {
    pub chaves: BTreeMap<&'static str, Entrada>,
    pub avisos: Vec<String>,
}

/// Como celebrar o fim de um turno do Claude (decisão 0003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModoCelebracao {
    Proporcional,
    SempreGrande,
    Discreta,
    Desligada,
}

/// Os pesos e os limites da pontuação (decisão 0074).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pesos {
    pub por_minuto: f64,
    pub por_ferramenta_de_trabalho: f64,
    pub por_outra_ferramenta: f64,
    pub por_arquivo: f64,
    pub por_subagente: f64,
    /// A pontuação nunca passa disto.
    pub teto: f64,
    /// Do T2 em diante.
    pub t2: f64,
    /// Do T3 em diante.
    pub t3: f64,
}

impl Default for Pesos {
    fn default() -> Pesos {
        Pesos {
            por_minuto: 1.0,
            por_ferramenta_de_trabalho: 0.15,
            por_outra_ferramenta: 0.05,
            por_arquivo: 0.5,
            por_subagente: 1.0,
            teto: 20.0,
            t2: 4.0,
            t3: 12.0,
        }
    }
}

/// Visão tipada da configuração, usada pelo resto do código.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub skin: String,
    /// O tamanho do pet na tela.
    pub tamanho: Tamanho,
    pub celebracao_modo: ModoCelebracao,
    /// O T3 no máximo a cada tanto (minutos).
    pub intervalo_t3_min: f64,
    /// Origens de sessão que o cérebro acompanha (`cli`, `sdk-cli`, …).
    pub sessoes_origens: Vec<String>,
    /// A pontuação dos turnos.
    pub pontuacao: Pesos,
}

impl ConfigEfetiva {
    /// Aplica padrões, depois o conteúdo do arquivo (se houver), depois o
    /// ambiente. `ambiente` recebe o nome da variável (`PET_APARENCIA_SKIN`).
    pub fn carregar(arquivo: Option<&str>, ambiente: impl Fn(&str) -> Option<String>) -> Self {
        let mut avisos = Vec::new();
        let mut chaves = BTreeMap::new();
        for chave in CHAVES {
            let valor = validar(chave.tipo, chave.padrao)
                .expect("padrão de config inválido: erro de programação");
            chaves.insert(
                chave.caminho,
                Entrada {
                    valor,
                    origem: Origem::Padrao,
                },
            );
        }

        if let Some(texto) = arquivo {
            match texto.parse::<toml::Table>() {
                Ok(tabela) => aplicar_arquivo(&tabela, &mut chaves, &mut avisos),
                Err(erro) => avisos.push(format!(
                    "arquivo de config ignorado (TOML inválido): {}",
                    erro.message()
                )),
            }
        }

        for chave in CHAVES {
            let nome = nome_variavel(chave.caminho);
            if let Some(bruto) = ambiente(&nome) {
                match validar(chave.tipo, bruto.trim()) {
                    Ok(valor) => {
                        chaves.insert(
                            chave.caminho,
                            Entrada {
                                valor,
                                origem: Origem::Ambiente,
                            },
                        );
                    }
                    Err(motivo) => avisos.push(format!("{nome} ignorada: {motivo}")),
                }
            }
        }

        // O T2 vem antes do T3: um par trocado não vale (os dois voltam ao
        // padrão).
        let numero = |chaves: &BTreeMap<&str, Entrada>, c: &str| match chaves[c].valor {
            Valor::Numero(n) => n,
            _ => f64::NAN,
        };
        if numero(&chaves, "pontuacao.t2") > numero(&chaves, "pontuacao.t3") {
            avisos.push(
                "pontuacao.t2 maior que pontuacao.t3: os dois ficam no padrão (4 e 12)".into(),
            );
            for caminho in ["pontuacao.t2", "pontuacao.t3"] {
                let chave = CHAVES
                    .iter()
                    .find(|c| c.caminho == caminho)
                    .expect("chave conhecida");
                chaves.insert(
                    caminho,
                    Entrada {
                        valor: validar(chave.tipo, chave.padrao).expect("padrão válido"),
                        origem: Origem::Padrao,
                    },
                );
            }
        }

        ConfigEfetiva { chaves, avisos }
    }

    /// Número efetivo de uma chave numérica conhecida.
    ///
    /// # Panics
    /// Se a chave não existir ou não for número (erro de programação).
    pub fn numero(&self, caminho: &str) -> f64 {
        match &self.chaves[caminho].valor {
            Valor::Numero(n) => *n,
            _ => panic!("{caminho} não é número"),
        }
    }

    /// Texto efetivo de uma chave conhecida.
    ///
    /// # Panics
    /// Se a chave não existir em `CHAVES` (erro de programação, coberto por
    /// teste).
    pub fn texto(&self, caminho: &str) -> &str {
        match &self.chaves[caminho].valor {
            Valor::Texto(texto) => texto,
            _ => panic!("{caminho} não é texto"),
        }
    }

    /// Itens efetivos de uma chave de lista conhecida.
    ///
    /// # Panics
    /// Se a chave não existir ou não for lista (erro de programação).
    pub fn lista(&self, caminho: &str) -> &[String] {
        match &self.chaves[caminho].valor {
            Valor::Lista(itens) => itens,
            _ => panic!("{caminho} não é lista"),
        }
    }

    pub fn config(&self) -> Config {
        let modo = match self.texto("celebracao.modo") {
            "sempre_grande" => ModoCelebracao::SempreGrande,
            "discreta" => ModoCelebracao::Discreta,
            "desligada" => ModoCelebracao::Desligada,
            _ => ModoCelebracao::Proporcional,
        };
        Config {
            skin: self.texto("aparencia.skin").to_owned(),
            tamanho: Tamanho::de_texto(self.texto("aparencia.tamanho")).unwrap_or_default(),
            celebracao_modo: modo,
            intervalo_t3_min: self.numero("celebracao.intervalo_t3_min"),
            sessoes_origens: self.lista("sessoes.origens").to_vec(),
            pontuacao: Pesos {
                por_minuto: self.numero("pontuacao.por_minuto"),
                por_ferramenta_de_trabalho: self.numero("pontuacao.por_ferramenta_de_trabalho"),
                por_outra_ferramenta: self.numero("pontuacao.por_outra_ferramenta"),
                por_arquivo: self.numero("pontuacao.por_arquivo"),
                por_subagente: self.numero("pontuacao.por_subagente"),
                teto: self.numero("pontuacao.teto"),
                t2: self.numero("pontuacao.t2"),
                t3: self.numero("pontuacao.t3"),
            },
        }
    }
}

/// `aparencia.skin` → `PET_APARENCIA_SKIN`.
pub fn nome_variavel(caminho: &str) -> String {
    format!("PET_{}", caminho.replace('.', "_").to_uppercase())
}

fn aplicar_arquivo(
    tabela: &toml::Table,
    chaves: &mut BTreeMap<&'static str, Entrada>,
    avisos: &mut Vec<String>,
) {
    let mut folhas = Vec::new();
    coletar_folhas("", tabela, &mut folhas);
    for (caminho, valor) in folhas {
        let Some(chave) = CHAVES.iter().find(|c| c.caminho == caminho) else {
            avisos.push(format!(
                "chave desconhecida no arquivo de config: {caminho}"
            ));
            continue;
        };
        let validado = match (chave.tipo, valor) {
            (Tipo::ListaDeIds, toml::Value::Array(itens)) => {
                match itens
                    .iter()
                    .map(toml::Value::as_str)
                    .collect::<Option<Vec<_>>>()
                {
                    Some(textos) => validar_lista(&textos),
                    None => Err("esperava uma lista de textos entre aspas".to_owned()),
                }
            }
            (Tipo::ListaDeIds, _) => Err("esperava uma lista, como [\"cli\"]".to_owned()),
            (Tipo::Numero { .. }, toml::Value::Integer(n)) => validar(chave.tipo, &n.to_string()),
            (Tipo::Numero { .. }, toml::Value::Float(n)) => validar(chave.tipo, &n.to_string()),
            (Tipo::Numero { .. }, _) => Err("esperava um número, como 0.5".to_owned()),
            (_, toml::Value::String(texto)) => validar(chave.tipo, texto),
            _ => Err("esperava texto entre aspas".to_owned()),
        };
        match validado {
            Ok(valor) => {
                chaves.insert(
                    chave.caminho,
                    Entrada {
                        valor,
                        origem: Origem::Arquivo,
                    },
                );
            }
            Err(motivo) => avisos.push(format!("{caminho} ignorada: {motivo}")),
        }
    }
}

fn coletar_folhas<'a>(
    prefixo: &str,
    tabela: &'a toml::Table,
    folhas: &mut Vec<(String, &'a toml::Value)>,
) {
    for (nome, valor) in tabela {
        let caminho = if prefixo.is_empty() {
            nome.clone()
        } else {
            format!("{prefixo}.{nome}")
        };
        match valor {
            toml::Value::Table(sub) => coletar_folhas(&caminho, sub, folhas),
            _ => folhas.push((caminho, valor)),
        }
    }
}

fn eh_id(texto: &str) -> bool {
    !texto.is_empty()
        && texto.len() <= 40
        && texto
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Lista de ids: cada item validado, sem repetição, até [`MAX_ITENS`].
fn validar_lista(itens: &[&str]) -> Result<Valor, String> {
    if itens.len() > MAX_ITENS {
        return Err(format!("lista com mais de {MAX_ITENS} itens"));
    }
    let mut lista: Vec<String> = Vec::with_capacity(itens.len());
    for item in itens {
        let item = item.trim();
        if !eh_id(item) {
            return Err(format!(
                "«{item}» não é um identificador ([a-z0-9_-], até 40)"
            ));
        }
        if !lista.iter().any(|x| x == item) {
            lista.push(item.to_owned());
        }
    }
    Ok(Valor::Lista(lista))
}

fn validar(tipo: Tipo, texto: &str) -> Result<Valor, String> {
    match tipo {
        Tipo::ListaDeIds => {
            let itens: Vec<&str> = texto
                .split(',')
                .map(str::trim)
                .filter(|i| !i.is_empty())
                .collect();
            validar_lista(&itens)
        }
        Tipo::Id => {
            if eh_id(texto) {
                Ok(Valor::Texto(texto.to_owned()))
            } else {
                Err(format!(
                    "«{texto}» não é um identificador ([a-z0-9_-], até 40)"
                ))
            }
        }
        Tipo::Opcao(opcoes) => {
            if opcoes.contains(&texto) {
                Ok(Valor::Texto(texto.to_owned()))
            } else {
                Err(format!(
                    "«{texto}» não é uma das opções: {}",
                    opcoes.join(", ")
                ))
            }
        }
        Tipo::Numero { min, max } => match texto.parse::<f64>() {
            Ok(n) if n.is_finite() && (min..=max).contains(&n) => Ok(Valor::Numero(n)),
            Ok(_) => Err(format!("«{texto}» fora da faixa de {min} a {max}")),
            Err(_) => Err(format!("«{texto}» não é um número (use ponto: 0.5)")),
        },
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn sem_ambiente(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn padroes_sem_arquivo_nem_ambiente() {
        let c = ConfigEfetiva::carregar(None, sem_ambiente);
        assert_eq!(c.texto("aparencia.skin"), "zeca");
        assert_eq!(c.chaves["aparencia.skin"].origem, Origem::Padrao);
        assert_eq!(c.config().celebracao_modo, ModoCelebracao::Proporcional);
        assert!(c.avisos.is_empty());
    }

    #[test]
    fn todo_padrao_passa_na_propria_validacao() {
        for chave in CHAVES {
            assert!(
                validar(chave.tipo, chave.padrao).is_ok(),
                "{}",
                chave.caminho
            );
        }
    }

    #[test]
    fn arquivo_vence_padrao_e_ambiente_vence_arquivo() {
        let arquivo = "[aparencia]\nskin = \"_teste\"\n[celebracao]\nmodo = \"discreta\"\n";
        let ambiente = |nome: &str| (nome == "PET_CELEBRACAO_MODO").then(|| "desligada".to_owned());
        let c = ConfigEfetiva::carregar(Some(arquivo), ambiente);
        assert_eq!(c.texto("aparencia.skin"), "_teste");
        assert_eq!(c.chaves["aparencia.skin"].origem, Origem::Arquivo);
        assert_eq!(c.config().celebracao_modo, ModoCelebracao::Desligada);
        assert_eq!(c.chaves["celebracao.modo"].origem, Origem::Ambiente);
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    }

    #[test]
    fn valor_invalido_vira_aviso_e_mantem_a_camada_anterior() {
        let arquivo = "[celebracao]\nmodo = \"exagerada\"\n";
        let ambiente = |nome: &str| (nome == "PET_APARENCIA_SKIN").then(|| "Zeca!".to_owned());
        let c = ConfigEfetiva::carregar(Some(arquivo), ambiente);
        assert_eq!(c.texto("celebracao.modo"), "proporcional");
        assert_eq!(c.texto("aparencia.skin"), "zeca");
        assert_eq!(c.avisos.len(), 2, "{:?}", c.avisos);
    }

    #[test]
    fn chave_desconhecida_e_tipo_errado_viram_aviso() {
        let arquivo = "[aparencia]\ncor = \"azul\"\nskin = 3\n";
        let c = ConfigEfetiva::carregar(Some(arquivo), sem_ambiente);
        assert_eq!(c.texto("aparencia.skin"), "zeca");
        assert_eq!(c.avisos.len(), 2, "{:?}", c.avisos);
        assert!(c.avisos.iter().any(|a| a.contains("aparencia.cor")));
    }

    #[test]
    fn toml_quebrado_nao_derruba() {
        let c = ConfigEfetiva::carregar(Some("[aparencia\nskin ="), sem_ambiente);
        assert_eq!(c.texto("aparencia.skin"), "zeca");
        assert_eq!(c.avisos.len(), 1);
    }

    #[test]
    fn origens_das_sessoes() {
        let c = ConfigEfetiva::carregar(None, sem_ambiente);
        assert_eq!(c.config().sessoes_origens, vec!["cli"]);
        let arquivo = "[sessoes]\norigens = [\"cli\", \"sdk-cli\", \"cli\"]\n";
        let c = ConfigEfetiva::carregar(Some(arquivo), sem_ambiente);
        assert_eq!(c.config().sessoes_origens, vec!["cli", "sdk-cli"]);
        assert_eq!(c.chaves["sessoes.origens"].origem, Origem::Arquivo);
        let ambiente =
            |nome: &str| (nome == "PET_SESSOES_ORIGENS").then(|| " cli , claude-vscode".into());
        let c = ConfigEfetiva::carregar(Some(arquivo), ambiente);
        assert_eq!(c.config().sessoes_origens, vec!["cli", "claude-vscode"]);
        // Vazio é uma escolha válida: nenhuma sessão conta.
        let c = ConfigEfetiva::carregar(None, |n: &str| {
            (n == "PET_SESSOES_ORIGENS").then(String::new)
        });
        assert!(c.config().sessoes_origens.is_empty());
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(
            json["chaves"]["sessoes.origens"]["valor"],
            serde_json::json!([])
        );
    }

    #[test]
    fn origens_invalidas_viram_aviso() {
        for arquivo in [
            "[sessoes]\norigens = \"cli\"\n",
            "[sessoes]\norigens = [\"CLI\"]\n",
            "[sessoes]\norigens = [1]\n",
        ] {
            let c = ConfigEfetiva::carregar(Some(arquivo), sem_ambiente);
            assert_eq!(c.config().sessoes_origens, vec!["cli"], "{arquivo}");
            assert_eq!(c.avisos.len(), 1, "{arquivo}: {:?}", c.avisos);
        }
        let ambiente = |nome: &str| (nome == "PET_SESSOES_ORIGENS").then(|| "cli,sdk cli".into());
        let c = ConfigEfetiva::carregar(None, ambiente);
        assert_eq!(c.config().sessoes_origens, vec!["cli"]);
        assert_eq!(c.avisos.len(), 1);
    }

    #[test]
    fn tamanho_do_pet() {
        let c = ConfigEfetiva::carregar(None, sem_ambiente);
        assert_eq!(c.config().tamanho, Tamanho::Normal);
        assert_eq!(c.chaves["aparencia.tamanho"].origem, Origem::Padrao);
        let arquivo = "[aparencia]\ntamanho = \"pequeno\"\n";
        let c = ConfigEfetiva::carregar(Some(arquivo), sem_ambiente);
        assert_eq!(c.config().tamanho, Tamanho::Pequeno);
        assert_eq!(c.chaves["aparencia.tamanho"].origem, Origem::Arquivo);
        assert_eq!(c.config().skin, "zeca", "a skin não muda");
        let ambiente = |nome: &str| (nome == "PET_APARENCIA_TAMANHO").then(|| "grande".into());
        assert_eq!(
            ConfigEfetiva::carregar(Some(arquivo), ambiente)
                .config()
                .tamanho,
            Tamanho::Grande
        );
        let ruim = "[aparencia]\ntamanho = \"gigante\"\n";
        let c = ConfigEfetiva::carregar(Some(ruim), sem_ambiente);
        assert_eq!(c.config().tamanho, Tamanho::Normal);
        assert_eq!(c.avisos.len(), 1, "{:?}", c.avisos);
    }

    #[test]
    fn pesos_da_pontuacao_com_faixa_e_t2_antes_do_t3() {
        let c = ConfigEfetiva::carregar(None, sem_ambiente);
        assert_eq!(c.config().pontuacao, Pesos::default());
        assert_eq!(c.config().intervalo_t3_min, 10.0);
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
        let arquivo =
            "[pontuacao]\npor_arquivo = 1\nt2 = 3.5\n[celebracao]\nintervalo_t3_min = 0\n";
        let ambiente =
            |nome: &str| (nome == "PET_PONTUACAO_POR_MINUTO").then(|| " 2.5 ".to_owned());
        let c = ConfigEfetiva::carregar(Some(arquivo), ambiente);
        let p = c.config().pontuacao;
        assert_eq!((p.por_arquivo, p.t2, p.por_minuto), (1.0, 3.5, 2.5));
        assert_eq!(c.config().intervalo_t3_min, 0.0);
        assert_eq!(c.chaves["pontuacao.por_minuto"].origem, Origem::Ambiente);
        assert!(c.avisos.is_empty(), "{:?}", c.avisos);
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["chaves"]["pontuacao.t2"]["valor"], 3.5);
        for ruim in [
            "[pontuacao]\npor_arquivo = -1\n",
            "[pontuacao]\npor_arquivo = \"muito\"\n",
            "[pontuacao]\nteto = 1e9\n",
            "[celebracao]\nintervalo_t3_min = \"nan\"\n",
        ] {
            let c = ConfigEfetiva::carregar(Some(ruim), sem_ambiente);
            assert_eq!(c.config().pontuacao, Pesos::default(), "{ruim}");
            assert_eq!(c.avisos.len(), 1, "{ruim}: {:?}", c.avisos);
        }
        let ambiente = |nome: &str| (nome == "PET_PONTUACAO_TETO").then(|| "1,5".to_owned());
        assert_eq!(ConfigEfetiva::carregar(None, ambiente).avisos.len(), 1);
        // T2 depois do T3: os dois voltam ao padrão.
        let trocado = "[pontuacao]\nt2 = 15\nt3 = 10\n";
        let c = ConfigEfetiva::carregar(Some(trocado), sem_ambiente);
        assert_eq!(
            (c.config().pontuacao.t2, c.config().pontuacao.t3),
            (4.0, 12.0)
        );
        assert_eq!(c.avisos.len(), 1, "{:?}", c.avisos);
    }

    #[test]
    fn o_compose_repassa_toda_chave_do_config() {
        // Decisão 0046: as chaves que o ambiente pode trocar só chegam ao
        // container se o compose as repassar.
        let compose = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docker-compose.yml"),
        )
        .expect("docker-compose.yml");
        for chave in CHAVES {
            let nome = nome_variavel(chave.caminho);
            assert!(
                compose.contains(&format!("      {nome}:")),
                "{nome} falta no environment do docker-compose.yml"
            );
        }
    }

    #[test]
    fn nome_da_variavel_de_ambiente() {
        assert_eq!(nome_variavel("aparencia.skin"), "PET_APARENCIA_SKIN");
    }

    #[test]
    fn serializa_valor_e_origem() {
        let c = ConfigEfetiva::carregar(None, sem_ambiente);
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["chaves"]["aparencia.skin"]["valor"], "zeca");
        assert_eq!(json["chaves"]["aparencia.skin"]["origem"], "padrao");
    }
}
