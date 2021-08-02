use std::{
    collections::hash_map::DefaultHasher,
    fs::File,
    hash::{Hash, Hasher},
    io::{Read, Write},
    net::TcpStream,
    sync::mpsc::{self, channel, Sender},
    thread,
};

use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::{Aes128Gcm, Key, Nonce};
use egui::{epaint, FontDefinitions};
use epi::Frame;
use font_loader::system_fonts;
use mpsc::Receiver;

struct Ap {
    cli: TcpStream,
    nam: String,
    text: String,
    reced: Vec<(String, String)>,
    rer: Receiver<String>,
    uspace: bool,
    key: Aes128Gcm,
    font: FontDefinitions,
}
//split message into name and content
fn split_meggage(raw: &String) -> (String, String) {
    let pos = raw.find(':').expect("Can't found ':' !");
    (raw[0..pos].to_string(), raw[pos + 1..].to_string())
}

impl epi::App for Ap {
    fn name(&self) -> &str {
        "TheClient"
    }
    fn update(&mut self, ctx: &egui::CtxRef, _: &mut Frame<'_>) {
        if let Ok(msg) = self.rer.try_recv() {
            //received message
            println!("{}", msg);
            self.reced.push(split_meggage(&msg));
            if self.reced.len() > 50 {
                self.reced.remove(0);
            }
        }
        ctx.set_fonts(self.font.clone());
        egui::TopBottomPanel::bottom("Sender").show(&ctx, |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.text).desired_width(ui.available_size().x));
            ui.add(egui::Checkbox::new(&mut self.uspace, "Send By Enter"));
            let send = ui.add(egui::Button::new("Send"));
            if (send.clicked() || (self.uspace && ui.input().key_down(egui::Key::Enter)))
                && self.text.trim() != ""
            //sending message & message isn't empty
            {
                let nonce = Nonce::from_slice(b"YesiqueNonce");
                let ct = self
                    .key
                    .encrypt(
                        nonce,
                        format!("{} : {}", &self.nam, &self.text.trim_end()).as_bytes(),
                    )
                    .expect("encryption failure!");
                self.cli.write((base64::encode(ct)).as_bytes()).unwrap();
                self.text.clear();
            }
        });
        egui::CentralPanel::default().show(&ctx, |ui| {
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                self.reced.iter().for_each(|x| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(&x.0)
                                .text_color(egui::color::Rgba::from_rgb(0.6, 0.8, 0.2)),
                        ); //name
                        ui.add(egui::Label::new(" : "));
                        ui.add(egui::Label::new(&x.1).wrap(true)); //text
                    });
                });
            });
        });
    }
}
fn read_msg(tx: Sender<String>, mut cli: TcpStream, key: Aes128Gcm) {
    let nonce = Nonce::from_slice(b"YesiqueNonce");
    loop {
        let mut rec = Vec::new();
        let mut buf;
        loop {
            buf = vec![0; 511];
            cli.read(buf.as_mut_slice()).expect("Server is offline!");
            rec.append(&mut (buf.clone()));
            if buf[510] == 0 {
                break;
            }
        }
        let message = rec.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
        let t = String::from_utf8(message).expect("Received a non-utf8 string");
        if let Ok(pt) = key.decrypt(
            nonce,
            base64::decode(&t)
                .expect("Unable to decode by Base64")
                .as_ref(),
        ) {
            let t = String::from_utf8(pt).expect("Received a non-utf8 string");
            tx.send(t).unwrap();
        }
    }
}
fn main() -> std::io::Result<()> {
    //--------------------load option-------------------------
    let mut buf = String::new();
    File::open("config.toml")
        .expect("\n\nCan't found config.toml.\nThe config needs:key,address,font,name,size.\n\n")
        .read_to_string(&mut buf)
        .expect("The config isn't an utf-8 file");
    let settings = buf
        .parse::<toml::Value>()
        .expect("Error when parsing options");
    //-------------------prepare multi-thread-----------------
    let (tx, rx) = channel();
    //-------------------load font----------------------------
    let property = system_fonts::FontPropertyBuilder::new()
        .family(
            settings["font"]
                .as_str()
                .expect("font is not a string!")
                .trim(),
        )
        .build();
    let (font, _) = system_fonts::get(&property).expect(&format!(
        "fail to get {}!",
        settings["font"].as_str().unwrap()
    ));
    let size = settings["size"]
        .as_integer()
        .expect("size is not a number!");
    let mut fonts = epaint::text::FontDefinitions::default();
    fonts.family_and_size.insert(
        epaint::text::TextStyle::Button,
        (epaint::text::FontFamily::Proportional, 20.0),
    );
    fonts
        .font_data
        .insert("TheFont".to_owned(), std::borrow::Cow::Owned(font));
    fonts.fonts_for_family.insert(
        epaint::text::FontFamily::Proportional,
        vec![
            "TheFont".to_owned(),
            "TheFont".to_owned(),
            "TheFont".to_owned(),
            "TheFont".to_owned(),
        ],
    );
    fonts.family_and_size.insert(
        epaint::text::TextStyle::Body,
        (epaint::text::FontFamily::Proportional, size as f32),
    );
    //-------------------link to server-----------------------
    let client = TcpStream::connect(settings["address"].as_str().unwrap().trim())?;
    let ts = client.try_clone().expect("Error when cloneing client");
    //-------------------prepare encryption-------------------
    let mut ss = base64::encode(settings["key"].as_str().unwrap().trim());
    let mut hasher = DefaultHasher::new();
    ss.hash(&mut hasher);
    ss = hasher.finish().to_string();
    ss = ss + "0000000000000000";
    let key = Key::from_slice(&ss.as_bytes()[0..16]);
    let cipher = Aes128Gcm::new(key);
    //---------------------------------------------------------
    thread::spawn(|| read_msg(tx, ts, cipher)); //reading message

    let key = Key::from_slice(&ss.as_bytes()[0..16]); //generate key
    let cipher = Aes128Gcm::new(key);

    eframe::run_native(
        Box::new(Ap {
            cli: client,
            nam: settings["name"].as_str().unwrap().trim().to_string(),
            text: String::new(),
            reced: Vec::<(String, String)>::new(),
            rer: rx,
            font: fonts,
            key: cipher,
            uspace: false,
        }),
        epi::NativeOptions::default(),
    );
}
