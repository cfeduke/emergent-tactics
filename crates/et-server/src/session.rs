//! One WebSocket connection = one session that can host a sequence of games.

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use et_engine::{Action, GameState, Outcome, Player, PublicEntity, StatusKind};
use et_protocol::ws::{ArchetypeInfo, CandidateInfo, CardInfo, ClientMessage, ServerMessage, StatChange, StatusInfo};
use et_search::{Difficulty, Evaluator, RolloutEvaluator, SearchResult, search};
use futures_util::{SinkExt, StreamExt};
use rand::{Rng, SeedableRng, seq::IndexedRandom};
use rand_chacha::ChaCha8Rng;

use crate::AppState;

/// Who controls a seat.
#[derive(Clone)]
enum Controller {
    Human,
    Random,
    Search(Difficulty),
}

impl Controller {
    fn from_name(name: &str) -> Option<Controller> {
        if name == "random" {
            return Some(Controller::Random);
        }
        Difficulty::by_name(name).map(Controller::Search)
    }
    fn name(&self) -> String {
        match self {
            Controller::Human => "human".into(),
            Controller::Random => "random".into(),
            Controller::Search(d) => d.name.clone(),
        }
    }
}

pub fn difficulty_names() -> Vec<String> {
    std::iter::once("random".to_string()).chain(Difficulty::presets().into_iter().map(|d| d.name)).collect()
}

struct Game {
    state: GameState,
    rng: ChaCha8Rng,
    controllers: [Controller; 2],
    human: Option<Player>,
}

impl Game {
    fn controller(&self, p: Player) -> &Controller {
        &self.controllers[p.index()]
    }
}

type Sink = futures_util::stream::SplitSink<WebSocket, Message>;

async fn send(sink: &mut Sink, msg: &ServerMessage) -> bool {
    match serde_json::to_string(msg) {
        Ok(text) => sink.send(Message::Text(text.into())).await.is_ok(),
        Err(e) => {
            tracing::error!(error = %e, "serialize");
            false
        }
    }
}

pub async fn run(socket: WebSocket, app: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();
    let mut game: Option<Game> = None;

    let catalog = ServerMessage::Catalog {
        cards: CardInfo::all(),
        statuses: StatusInfo::all(),
        archetypes: ArchetypeInfo::all(),
        difficulties: difficulty_names(),
        library_fingerprint: et_engine::library_fingerprint_hex(),
    };
    if !send(&mut sink, &catalog).await {
        return;
    }

    while let Some(Ok(msg)) = stream.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let parsed: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                send(&mut sink, &ServerMessage::Error { message: format!("bad message: {e}") }).await;
                continue;
            }
        };
        match parsed {
            ClientMessage::NewGame { difficulty, seat } => {
                let Some(ai) = Controller::from_name(&difficulty) else {
                    send(&mut sink, &ServerMessage::Error { message: format!("unknown difficulty {difficulty}") }).await;
                    continue;
                };
                let controllers = match seat {
                    Player::First => [Controller::Human, ai],
                    Player::Second => [ai, Controller::Human],
                };
                game = Some(start_game(&mut sink, controllers, Some(seat)).await);
            }
            ClientMessage::Spectate { first, second } => {
                let (Some(a), Some(b)) = (Controller::from_name(&first), Controller::from_name(&second)) else {
                    send(&mut sink, &ServerMessage::Error { message: "unknown difficulty".into() }).await;
                    continue;
                };
                game = Some(start_game(&mut sink, [a, b], None).await);
            }
            ClientMessage::Act { action } => {
                let Some(g) = game.as_mut() else {
                    send(&mut sink, &ServerMessage::Error { message: "no game in progress".into() }).await;
                    continue;
                };
                if g.human != Some(g.state.to_move) {
                    send(&mut sink, &ServerMessage::Error { message: "not your turn".into() }).await;
                    continue;
                }
                if !g.state.is_legal(&action) {
                    send(&mut sink, &ServerMessage::Error { message: format!("illegal action {action:?}") }).await;
                    continue;
                }
                let player = g.state.to_move;
                let before = g.state.clone();
                g.state.apply(&action, &mut g.rng);
                let (changes, summary) = summarize(&before, &g.state, player, &action, g.human);
                if !send(&mut sink, &ServerMessage::Acted { player, action, changes, summary }).await {
                    return;
                }
                if !send_state(&mut sink, g).await {
                    return;
                }
            }
        }
        if let Some(g) = game.as_mut() {
            if !drive_ai(&mut sink, g, app.ai_delay_ms).await {
                return;
            }
        }
    }
}

async fn start_game(sink: &mut Sink, controllers: [Controller; 2], human: Option<Player>) -> Game {
    let seed: u64 = rand::rng().random();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let state = GameState::new(&mut rng);
    let g = Game { state, rng, controllers, human };
    send(
        sink,
        &ServerMessage::GameStarted {
            game_id: format!("{seed:016x}"),
            you: human,
            first: g.controller(Player::First).name(),
            second: g.controller(Player::Second).name(),
        },
    )
    .await;
    send_state(sink, &g).await;
    g
}

/// Observation goes from the human's perspective if there is one, else from
/// the player to move so spectators see the acting hand.
async fn send_state(sink: &mut Sink, g: &Game) -> bool {
    let viewer = g.human.unwrap_or(g.state.to_move);
    let legal = if g.human == Some(g.state.to_move) { g.state.legal_actions() } else { Vec::new() };
    if !send(sink, &ServerMessage::State { observation: g.state.observe(viewer), legal }).await {
        return false;
    }
    if let Some(outcome) = g.state.outcome() {
        return send(sink, &ServerMessage::GameOver { outcome }).await;
    }
    true
}

/// Keep taking AI turns until a human must act or the game ends.
async fn drive_ai(sink: &mut Sink, g: &mut Game, delay_ms: u64) -> bool {
    while g.state.outcome().is_none() && g.human != Some(g.state.to_move) {
        let player = g.state.to_move;
        let controller = g.controller(player).clone();
        let obs = g.state.observe(player);
        let legal = g.state.legal_actions();
        let seed: u64 = g.rng.random();

        let (action, decision) = match controller {
            Controller::Human => unreachable!(),
            Controller::Random => {
                let a = *legal.choose(&mut g.rng).expect("legal actions");
                (a, None)
            }
            Controller::Search(difficulty) => {
                let (legal2, d2) = (legal.clone(), difficulty.clone());
                let result: SearchResult = tokio::task::spawn_blocking(move || {
                    let mut rng = ChaCha8Rng::seed_from_u64(seed);
                    let evaluator: Box<dyn Evaluator> = Box::new(RolloutEvaluator { max_depth: 60 });
                    search(&obs, &legal2, &d2, evaluator.as_ref(), &mut rng)
                })
                .await
                .expect("search task");
                let candidates = result
                    .stats
                    .candidates
                    .iter()
                    .map(|c| CandidateInfo { action: c.action, prior: c.prior, visits: c.visits, value: c.value })
                    .collect();
                let decision = ServerMessage::AiDecision {
                    player,
                    difficulty: difficulty.name.clone(),
                    iterations: result.stats.iterations,
                    elapsed_ms: result.stats.elapsed_ms,
                    candidates,
                    chosen: result.action,
                };
                (result.action, Some(decision))
            }
        };

        if let Some(d) = decision
            && !send(sink, &d).await
        {
            return false;
        }
        let before = g.state.clone();
        g.state.apply(&action, &mut g.rng);
        let (changes, summary) = summarize(&before, &g.state, player, &action, g.human);
        if !send(sink, &ServerMessage::Acted { player, action, changes, summary }).await {
            return false;
        }
        if !send_state(sink, g).await {
            return false;
        }
        // Pace AI moves so the client can show each play.
        if g.state.outcome().is_none() {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }
    true
}

/// Structured and human-readable accounts of what an action changed,
/// from public state. Labels are relative to the human seat, or seat names
/// when spectating.
fn summarize(before: &GameState, after: &GameState, player: Player, action: &Action, human: Option<Player>) -> (Vec<StatChange>, Vec<String>) {
    let who = |p: Player| match human {
        Some(h) if h == p => "you",
        Some(_) => "opponent",
        None => match p {
            Player::First => "First",
            Player::Second => "Second",
        },
    };
    let mut changes = Vec::new();
    let mut lines = vec![match action {
        Action::Play(k) => format!("{} played {}", who(player), k.def().name),
        Action::EndTurn => format!("{} ended the turn", who(player)),
    }];
    for p in [Player::First, Player::Second] {
        let (b, a) = (PublicEntity::of(before.entity(p)), PublicEntity::of(after.entity(p)));
        let label = who(p);
        let mut change = |stat: &str, bv: i32, av: i32| {
            if bv != av {
                changes.push(StatChange { player: p, stat: stat.into(), before: bv, after: av });
                lines.push(format!("{label}: {stat} {bv} -> {av}"));
            }
        };
        change("life", b.lifepool, a.lifepool);
        change("block", b.block as i32, a.block as i32);
        let status = |e: &PublicEntity, k: StatusKind| e.statuses.iter().find(|(kk, _)| *kk == k).map(|(_, n)| *n as i32).unwrap_or(0);
        for k in StatusKind::ALL {
            change(&format!("status:{:?}", k), status(&b, k), status(&a, k));
        }
        if matches!(action, Action::EndTurn) && after.to_move == p {
            change("resource", b.resource as i32, a.resource as i32);
        }
    }
    if let Some(o) = after.outcome() {
        lines.push(match o {
            Outcome::Win(w) => format!("{} wins", who(w)),
            Outcome::Draw => "draw".into(),
        });
    }
    (changes, lines)
}
