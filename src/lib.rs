use std::{io::{self, Write}, cmp::Ordering, time::Duration, thread, fmt::{Display, Formatter}};
use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;

pub type Hand = Vec<Card>;

pub const DECK_COUNT: u8 = 8;

#[derive(Clone, Copy)]
pub struct Card {
    pub face_up: bool,
    pub suit: Suit,
    pub value: u8,
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        if !self.face_up {
            f.write_str("##")?;
            return Ok(());
        }      

        let value_string = match self.value {
            1 => String::from("A"),
            2..=9 => format!("{}", self.value),
            10 => String::from("10"),
            11 => String::from("J"),
            12 => String::from("Q"),
            13 => String::from("K"),
            other => panic!("unexpected card value: {}", other),
        };

        /*
        let suit_str = match self.suit {
            Suit::Clubs => "C",
            Suit::Hearts => "H",
            Suit::Spades => "S",
            Suit::Diamonds => "D",
        };
        */

        f.write_str(&value_string)?;
        // f.write_str(suit_str)?;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub enum Suit {
    Clubs,
    Hearts,
    Spades,
    Diamonds,
} 

pub struct GameState {
    pub dealer_hand: Hand,
    pub player_hand: Hand,
    pub current_person: Person,
    pub surrendered: bool,
}

impl GameState {
    pub fn init(deck: &mut Vec<Card>) -> Self {
        let mut state = Self {
            dealer_hand: Vec::new(),
            player_hand: Vec::new(),
            current_person: Person::Player,
            surrendered: false,
        };

        let mut dealer_down_card = draw_card(deck);
        dealer_down_card.face_up = false;
        state.dealer_hand.push(dealer_down_card);

        let dealer_up_card = draw_card(deck);
        state.dealer_hand.push(dealer_up_card);

        for _ in 0..2 {
            state.player_hand.push(draw_card(deck));   
        }

        state
    }   
}

#[derive(Clone, Copy)]
pub enum Person {
    Player,
    Dealer,
}

impl Display for GameState {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        for card in self.dealer_hand.iter() {
            f.write_fmt(format_args!("{} ", card))?;    
        }   

        if self.dealer_hand[0].face_up {
            match eval_hand(self.dealer_hand.clone()) {
                HandValue::Hard(value) | HandValue::Soft(value) => {
                    f.write_fmt(format_args!(" : {value}"))?;    
                },
                HandValue::Bust(value) => {
                    f.write_fmt(format_args!(" : {value} BUST"))?;   
                },
                HandValue::BlackJack => {
                    f.write_str(" : BLACKJACK")?;   
                }
            }
        }

        f.write_str("\n\n")?;

        for card in self.player_hand.iter() {
            f.write_fmt(format_args!("{} ", card))?;   
        }

        match eval_hand(self.player_hand.clone()) {
            HandValue::Hard(value) | HandValue::Soft(value) => {
                f.write_fmt(format_args!(" : {value}"))?;    
            },
            HandValue::Bust(value) => {
                f.write_fmt(format_args!(" : {value} BUST"))?;   
            },
            HandValue::BlackJack => {
                f.write_str(" : BLACKJACK")?;   
            }
        }

        f.write_str("\n")?;

        Ok(())
    }
}

pub enum TurnAction {
    Hit,
    Stand,
    DoubleDown,
    Surrender,
}

#[derive(Debug, Clone, Copy)]
pub enum HandValue {
    Hard(u8),
    Soft(u8),
    Bust(u8),
    BlackJack,
}

pub fn eval_hand(hand: Hand) -> HandValue {
    let mut ace_count = 
        hand.iter()
        .filter(|x| {
            x.value == 1
        })
        .count();

    let mut total =
        hand.iter()
        .fold(0, |acc, x| {
            acc + match x.value {
                1 => 11,
                11..=13 => 10,
                other => other,
            }
        });

    if hand.len() == 2 && total == 21 {
        return HandValue::BlackJack;   
    }

    while ace_count > 0 {
        if total <= 21 {
            return HandValue::Soft(total);    
        }   

        total -= 10;
        ace_count -= 1;
    }

    if total > 21 {
        HandValue::Bust(total)
    } else {
        HandValue::Hard(total)   
    }
}

pub enum TurnResult {
    Unfinished,
    Doubled,
    Stood,
    Won,
    WonBlackJack,
    Draw,
    Loss,
    Surrendered,
    SurrenderedFinished,
}

pub fn draw_card(deck: &mut Vec<Card>) -> Card {
    let mut rng = SmallRng::from_entropy();
    let idx = rng.gen_range(0..deck.len());
    let card = deck[idx];
    deck.remove(idx);

    if deck.is_empty() {
        // refill deck
        
        for _ in 0..DECK_COUNT {
            for value in 1..=13 {
                deck.push(Card { face_up: true, suit: Suit::Clubs, value });   
                deck.push(Card { face_up: true, suit: Suit::Hearts, value });   
                deck.push(Card { face_up: true, suit: Suit::Spades, value });   
                deck.push(Card { face_up: true, suit: Suit::Diamonds, value });   
            }
        }
    }

    card
}

pub fn run_player_turn(state: &mut GameState, deck: &mut Vec<Card>, bank: &mut u32, bet_amount: u32) -> TurnResult {
    let can_double = bet_amount <= (*bank - bet_amount);

    let action;

    loop {
        if can_double {
            print!("Hit(F), Stand(J), Double(Y), Surrender(T)");
            let flush_result = io::stdout().flush();

            if let Err(err) = flush_result {
                eprintln!("An unexpected I/O error occured: {err}");
                continue;
            }

            let mut answer = String::new();
            io::stdin().read_line(&mut answer).unwrap_or_else(|err| {
                eprintln!("An unexpected I/O error occured: {err}");
                std::process::exit(1);
            });

            match answer.trim().to_ascii_lowercase().as_str() {
                "f" => action = TurnAction::Hit,
                "j" => action = TurnAction::Stand,
                "y" => action = TurnAction::DoubleDown,
                "t" => action = TurnAction::Surrender,
                other => {
                    println!("{other} is not a valid action");
                    continue;
                },
            }

            break;
        } else {
            print!("Hit(F), Stand(J), Surrender(T)");
            let flush_result = io::stdout().flush();

            if let Err(err) = flush_result {
                eprintln!("An unexpected I/O error occured: {err}");
                continue;
            }

            let mut answer = String::new();
            io::stdin().read_line(&mut answer).unwrap_or_else(|err| {
                eprintln!("An unexpected I/O error occured: {err}");
                std::process::exit(1);
            });

            match answer.trim().to_ascii_lowercase().as_str() {
                "f" => action = TurnAction::Hit,
                "j" => action = TurnAction::Stand,
                "t" => action = TurnAction::Surrender,
                other => {
                    println!("{other} is not a valid action");
                    continue;
                },
            }

            break;
        }
    }

    match action {
        TurnAction::Hit => {
            state.player_hand.push(draw_card(deck));

            match eval_hand(state.player_hand.clone()) {
                HandValue::Bust(_) => TurnResult::Stood,
                _ => TurnResult::Unfinished,
            }
        },
        TurnAction::Stand => TurnResult::Stood,
        TurnAction::DoubleDown => {
            state.player_hand.push(draw_card(deck));
            TurnResult::Doubled
        },
        TurnAction::Surrender => TurnResult::Surrendered,
    }
}

pub fn run_dealer_turn(state: &mut GameState, deck: &mut Vec<Card>) -> TurnResult {
    thread::sleep(Duration::from_millis(1500));

    match eval_hand(state.dealer_hand.clone()) {
        HandValue::Hard(dealer_value) | HandValue::Soft(dealer_value) => {
            match dealer_value {
                3..=16 => {
                    state.dealer_hand.push(draw_card(deck));
                    TurnResult::Unfinished
                },
                17..=21 => {
                    if !state.surrendered {
                        match eval_hand(state.player_hand.clone()) {
                            HandValue::Hard(player_value) | HandValue::Soft(player_value) => {
                                match player_value.cmp(&dealer_value) {
                                    Ordering::Less => TurnResult::Loss,
                                    Ordering::Equal => TurnResult::Draw,
                                    Ordering::Greater => TurnResult::Won,
                                } 
                            },
                            HandValue::Bust(_) => TurnResult::Loss,
                            HandValue::BlackJack => TurnResult::WonBlackJack,
                        }   
                    } else {
                        TurnResult::SurrenderedFinished   
                    }
                },
                other => panic!("unexpected hand value: {}", other),
            }   
        },
        HandValue::Bust(_) => {
            if !state.surrendered {
                match eval_hand(state.player_hand.clone()) {
                    HandValue::Bust(_) => TurnResult::Loss,
                    HandValue::BlackJack => TurnResult::WonBlackJack,
                    _ => TurnResult::Won,
                } 
            } else {
                TurnResult::SurrenderedFinished   
            }
        },
        HandValue::BlackJack => {
            if !state.surrendered {
                match eval_hand(state.player_hand.clone()) {
                    HandValue::BlackJack => TurnResult::WonBlackJack,
                    _ => TurnResult::Loss,
                }   
            } else {
                TurnResult::SurrenderedFinished   
            }
        },
    }
}

pub fn run_turn(state: &mut GameState, deck: &mut Vec<Card>, bank: &mut u32, bet_amount: u32) -> TurnResult {
    match state.current_person {
        Person::Dealer => run_dealer_turn(state, deck),
        Person::Player => run_player_turn(state, deck, bank, bet_amount),
    }
}

pub fn run_hand(bank: &mut u32, deck: &mut Vec<Card>) {
    let mut bet_amount = get_bet(*bank);
    let mut state = GameState::init(deck);

    loop {
        print!("----------\n{state}");
        let flush_result = io::stdout().flush();

        if let Err(err) = flush_result {
            eprintln!("An unexpected I/O error occured: {err}");
            continue;
        }

        match run_turn(&mut state, deck, bank, bet_amount) {
            TurnResult::Unfinished => (),   
            TurnResult::Doubled => {
                bet_amount *= 2;
                state.current_person = Person::Dealer;
                state.dealer_hand[0].face_up = true;
            },
            TurnResult::Stood => {
                state.current_person = Person::Dealer;
                state.dealer_hand[0].face_up = true;
            },
            TurnResult::Won => {
                *bank += bet_amount;
                break;
            },
            TurnResult::WonBlackJack => {
                *bank += (bet_amount as f64 * 1.5f64) as u32;
                break;
            },
            TurnResult::Draw => break, 
            TurnResult::Loss => {
                *bank -= bet_amount;
                break;
            },
            TurnResult::Surrendered => {
                state.surrendered = true;
                state.current_person = Person::Dealer;
                state.dealer_hand[0].face_up = true;
            },
            TurnResult::SurrenderedFinished => {
                *bank -= (bet_amount as f64 * 0.5f64) as u32;
                break;
            },
        }
    }
}

pub fn get_bet(bank: u32) -> u32 {
    println!("Chips: {bank}");

    let mut bet_amount;

    loop {
        print!("What do you bet? ");
        let flush_result = io::stdout().flush();

        if let Err(err) = flush_result {
            eprintln!("An unexpected I/O error occured: {err}");
            continue;
        }

        let mut bet_string = String::new();
        io::stdin().read_line(&mut bet_string).unwrap_or_else(|err| {
            eprintln!("An unexpected I/O error occured: {err}");
            std::process::exit(1);
        });

        let bet_amount_result = bet_string.trim().parse::<u32>();

        bet_amount = if let Ok(value) = bet_amount_result {
            value   
        } else {
            println!("That's not a valid bet amount");
            continue;
        };

        if bet_amount > bank {
            println!("You don't have enough chips");
            continue;
        }

        if bet_amount < 1 {
            println!("Minimum bet is 1 chip");
            continue;
        }

        break;
    }

    bet_amount
}
