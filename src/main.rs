use once_cell::sync::Lazy;

use teloxide::prelude::*;
use teloxide::types::Message;
use teloxide::utils::command::BotCommands;
extern crate dotenv;
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind, Write};
use std::{env, fs};
use tokio::sync::Mutex;

// ToDo: add translationEngine struct and interface for generalization of translation

#[derive(Serialize, Deserialize, Debug)]
struct User {
    user_id: UserId,
    source_language: Language,
    target_language: Language,
}
impl User {
    fn clone(&self) -> User {
        User {
            user_id: self.user_id,
            source_language: self.source_language.clone(),
            target_language: self.target_language.clone(),
        }
    }
}

#[derive(Clone, BotCommands)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(
        description = "Register your source language and target language you want to translate to",
        parse_with = "split"
    )]
    Register {
        source_language: String,
        target_language: String,
    },
    #[command(description = "display your status.")]
    Status,
}

#[derive(Serialize, Deserialize)]
struct TranslationRequest {
    q: String,
    source_language: String,
    target_language: String,
}
#[derive(Serialize, Deserialize)]
struct TranslationResponse {
    translatedText: String,
}
struct UserStore {
    users: Vec<User>,
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Language {
    name: String,
    code: String,
}

static USER_STORE: Lazy<Mutex<UserStore>> = Lazy::new(|| Mutex::new(UserStore::new()));
#[tokio::main]
async fn main() {
    dotenv().ok();
    let bot = Bot::from_env();
    let user_store = USER_STORE.lock().await;
    println!("User store is {:?}", user_store.users);
    drop(user_store);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(message) = msg.text() {
            if message.starts_with('/') {
                let cmd = Command::parse(message, "TranslateBot").unwrap();
                return answer(bot, msg, cmd).await.map_err(Into::into);
            }

            let user_store = USER_STORE.lock().await;
            let user_opt = user_store.get_user(msg.from().unwrap().id);
            if let Some(user) = user_opt {
                handle_translation(&msg, &bot, user).await;
            } else {
                // auto translate as user has not been registered
                handle_translation(
                    &msg,
                    &bot,
                    &User {
                        user_id: msg.from().unwrap().id,
                        source_language: Language::new("auto".to_string()),
                        target_language: Language::new("en".to_string()),
                    },
                )
                .await;
            }
        }
        Ok(())
    })
    .await;
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Register {
            source_language,
            target_language,
        } => {
            match USER_STORE.lock().await.register(
                source_language.to_ascii_lowercase(),
                target_language.to_ascii_lowercase(),
                msg.from().unwrap().id,
            ) {
                Ok(message) => bot.send_message(msg.chat.id, message).await?,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Something went wrong".to_string())
                        .await?
                }
            }
        }
        Command::Status => {
            bot.send_message(
                msg.chat.id,
                USER_STORE.lock().await.user_status(msg.from().unwrap().id),
            )
            .await?
        }
    };

    Ok(())
}

async fn translate(translation_request: TranslationRequest) -> Result<String, Error> {
    let form = reqwest::multipart::Form::new()
        .text("q", translation_request.q)
        .text("source", translation_request.source_language)
        .text("target", translation_request.target_language);

    let client = reqwest::Client::new();
    let url = match env::var("LIBRETRANSLATE_API_URL") {
        Ok(url) => url,
        Err(_) => "https://libretranslate.com/translate".to_string(),
    };
    let resp = client.post(url).multipart(form).send().await.unwrap();
    let translation_response: TranslationResponse =
        serde_json::from_str(&resp.text().await.unwrap()).unwrap();

    return Ok(translation_response.translatedText);
}

async fn handle_translation(message: &Message, bot: &Bot, user: &User) {
    let translation_request = TranslationRequest {
        q: String::from(message.text().unwrap()),
        source_language: user.source_language.clone().code,
        target_language: user.target_language.clone().code,
    };
    let translation = translate(translation_request).await;
    match translation {
        Ok(translation) => {
            bot.send_message(message.chat.id, translation)
                .reply_to_message_id(message.id)
                .await
                .expect("Unable to send message");
        }
        Err(_) => {
            bot.send_message(message.chat.id, "Something went wrong".to_string())
                .await
                .expect("Unable to send message");
        }
    }
}

impl Language {
    fn new(name: String) -> Language {
        Language {
            name: name.clone(),
            code: match Language::retrieve_language_code(&name) {
                Ok(code) => code,
                Err(_) => "en".to_string(),
            },
        }
    }

    fn clone(&self) -> Language {
        Language {
            name: self.name.clone(),
            code: self.code.clone(),
        }
    }

    fn retrieve_language_code(language: &String) -> Result<String, Error> {
        match language.as_str() {
            "english" => Ok("en".to_string()),
            "thai" => Ok("th".to_string()),
            "french" => Ok("fr".to_string()),
            "german" => Ok("de".to_string()),
            "spanish" => Ok("es".to_string()),
            "italian" => Ok("it".to_string()),
            "russian" => Ok("ru".to_string()),
            "japanese" => Ok("ja".to_string()),
            "korean" => Ok("ko".to_string()),
            "chinese" => Ok("zh".to_string()),
            "arabic" => Ok("ar".to_string()),
            "dutch" => Ok("nl".to_string()),
            "polish" => Ok("pl".to_string()),
            "portuguese" => Ok("pt".to_string()),
            "swedish" => Ok("sv".to_string()),
            "turkish" => Ok("tr".to_string()),
            "vietnamese" => Ok("vi".to_string()),
            "bulgarian" => Ok("bg".to_string()),
            "czech" => Ok("cs".to_string()),
            "danish" => Ok("da".to_string()),
            "greek" => Ok("el".to_string()),
            "finnish" => Ok("fi".to_string()),
            "hebrew" => Ok("he".to_string()),
            "hindi" => Ok("hi".to_string()),
            "hungarian" => Ok("hu".to_string()),
            "indonesian" => Ok("id".to_string()),
            "latvian" => Ok("lv".to_string()),
            "norwegian" => Ok("no".to_string()),
            "romanian" => Ok("ro".to_string()),
            "slovak" => Ok("sk".to_string()),
            "slovenian" => Ok("sl".to_string()),
            "ukrainian" => Ok("uk".to_string()),
            "catalan" => Ok("ca".to_string()),
            "estonian" => Ok("et".to_string()),
            "persian" => Ok("fa".to_string()),
            "afrikaans" => Ok("af".to_string()),
            "albanian" => Ok("sq".to_string()),
            "amharic" => Ok("am".to_string()),
            "armenian" => Ok("hy".to_string()),
            "azerbaijani" => Ok("az".to_string()),
            "bengali" => Ok("bn".to_string()),
            "tagalog" => Ok("tl".to_string()),
            _ => Err(Error::new(ErrorKind::InvalidInput, "Invalid language")),
        }
    }
}

impl UserStore {
    fn new() -> UserStore {
        UserStore::load()
    }

    fn load() -> UserStore {
        match env::var("USERS_INFO_PATH") {
            Ok(path) => {
                match UserStore::ensure_file(&path) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Unable to create file");
                    }
                }
                let contents =
                    fs::read_to_string(&path).expect("Something went wrong reading the file");
                // my understanding here is that the above line would catch the error hence unwrapping here is alright
                let users = serde_json::from_str(&contents);
                match users {
                    Ok(users) => UserStore { users, path },
                    Err(_) => UserStore {
                        users: Vec::new(),
                        path,
                    },
                }
            }
            Err(_) => UserStore {
                users: Vec::new(),
                path: "".to_string(),
            },
        }
    }

    fn ensure_file(path: &String) -> Result<(), Error> {
        match fs::metadata(&path) {
            Ok(_) => {
                return Ok(());
            }
            Err(_) => {}
        }
        match fs::canonicalize(path) {
            Ok(path) => {
                println!("Path is {}", path.display());
                match fs::File::create(path) {
                    Ok(mut file) => {
                        file.write_all(b"[]").expect("Unable to write file");
                    }
                    Err(_) => {
                        println!("Unable to create file");
                    }
                }
            }
            Err(_) => {
                println!("Unable to canonicalize path");
            }
        }

        Ok(())
    }

    fn add_user(&mut self, user: User) {
        self.users.push(user.clone());
    }

    fn get_user(&self, user_id: UserId) -> Option<&User> {
        self.users.iter().find(|&user| user.user_id == user_id)
    }

    fn update_user(&mut self, user: User) {
        let index = self
            .users
            .iter()
            .position(|x| x.user_id == user.user_id)
            .unwrap();
        self.users[index] = user;
    }

    fn save(&self) {
        let serialized = serde_json::to_string(&self.users).unwrap();
        fs::write(self.path.clone(), serialized).expect("Unable to write file");
    }

    fn register(
        &mut self,
        source_language: String,
        target_language: String,
        user_id: UserId,
    ) -> Result<String, Error> {
        // check if user is registered
        let user = User {
            user_id,
            source_language: Language::new(source_language.clone()),
            target_language: Language::new(target_language.clone()),
        };

        if self.get_user(user_id).is_some() {
            self.update_user(user);
            self.save();
            return Ok(format!(
                "You have been updated to translate from {source_language} to {target_language}"
            )
            .to_string());
        }
        self.add_user(user);
        self.save();
        return Ok(format!(
            "You have been registered to translate from {source_language} to {target_language}"
        )
        .to_string());
    }

    fn user_status(&self, user_id: UserId) -> String {
        match self.get_user(user_id) {
            Some(user) => {
                format!(
                    "You are registered to translate from {source_language} to {target_language}",
                    source_language = user.source_language.name,
                    target_language = user.target_language.name
                )
            }
            None => {
                format!("You are not registered")
            }
        }
    }
}
