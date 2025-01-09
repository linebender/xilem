//! A demonstration of using variable fonts.

#![expect(elided_lifetimes_in_paths, reason = "Deferred: Noisy")]

use accesskit::{Node, Role};
use masonry::{
    event_loop_runner::{EventLoop, EventLoopBuilder, MasonryState},
    text::ArcStr,
    widget::{Flex, Label, RootWidget},
    AccessCtx, Action, AppDriver, BoxConstraints, DriverCtx, FontWeight, LayoutCtx, PaintCtx,
    Point, RegisterCtx, Size, UpdateCtx, Widget, WidgetId, WidgetPod,
};
use parley::StyleProperty;
use smallvec::SmallVec;
use vello::Scene;
use winit::{error::EventLoopError, window::Window};

struct Driver;

impl AppDriver for Driver {
    fn on_action(&mut self, _ctx: &mut DriverCtx<'_>, _widget_id: WidgetId, _action: Action) {}
    fn on_start(&mut self, state: &mut MasonryState) {
        let root = state.get_root();

        // We don't need the resulting family information
        drop(root.register_fonts(HAHMLET_VARIABLE.into()));
    }
}

const TEXT: &[&str] = &[
    "가각간갇갈갉갊갋감갑값갓갔강갖갗",
    "같갚갛개객갠갣갤갬갭갯갰갱갸갹갼",
    "걀걋걍걔걘걜걥거걱건걷걸걺검겁겂",
    "것겄겅겆겉겊겋게겍겐겔겜겝겟겠겡",
    "겨격겪견겯결겷겸겹겻겼경겿곁계곈",
    "곌곕곗곘고곡곤곧골곪곬곯곰곱곳공",
    "곶곹과곽관괄괆괌괍괏괐광괒괘괜괠",
    "괢괩괬괭괴괵괸괼굄굅굇굉교굔굘굠",
    "굡굣굥구국군굳굴굵굶굻굼굽굿궁궂",
    "궃궈궉권궐궜궝궤궷궸귀귁귄귈귐귑",
    "귓귕규균귤귬귭그극근귿글긁긂긇금",
    "급긋긍긏긑긓긔긘긩기긱긴긷길긺김",
    "깁깃깄깅깆깊까깍깎깐깔깖깜깝깟깠",
    "깡깥깨깩깬깯깰깸깹깻깼깽꺄꺅꺆꺌",
    "꺍꺠꺤꺼꺽꺾껀껄껌껍껏껐껑껓껕께",
    "껙껜껨껫껭껴껸껼꼇꼈꼉꼍꼐꼬꼭꼰",
    "꼲꼳꼴꼼꼽꼿꽁꽂꽃꽅꽈꽉꽌꽐꽜꽝",
    "꽤꽥꽨꽸꽹꾀꾄꾈꾐꾑꾕꾜꾸꾹꾼꿀",
    "꿇꿈꿉꿋꿍꿎꿏꿔꿘꿜꿨꿩꿰꿱꿴꿸",
    "뀀뀁뀄뀌뀐뀔뀜뀝뀨뀰뀼끄끅끈끊끌",
    "끎끓끔끕끗끙끝끠끤끼끽낀낄낌낍낏",
    "낐낑나낙낚난낟날낡낢남납낫났낭낮",
    "낯낱낳내낵낸낻낼냄냅냇냈냉냐냑냔",
    "냗냘냠냡냣냥냬너넉넋넌넏널넑넒넓",
    "넘넙넛넜넝넢넣네넥넨넫넬넴넵넷넸",
    "넹넾녀녁년녇녈념녑녔녕녘녜녠녱노",
    "녹논놀놁놂놈놉놋농놑높놓놔놘놜놥",
    "놨놰뇄뇌뇍뇐뇔뇜뇝뇟뇡뇨뇩뇬뇰뇸",
    "뇹뇻뇽누눅눈눋눌눍눔눕눗눙눝눠눴",
    "눼뉘뉜뉠뉨뉩뉴뉵뉻뉼늄늅늉느늑는",
    "늗늘늙늚늠늡늣능늦늧늪늫늬늰늴늼",
    "늿닁니닉닌닏닐닒님닙닛닝닞닠닢다",
    "닥닦단닫달닭닮닯닳담답닷닸당닺닻",
    "닽닿대댁댄댈댐댑댓댔댕댖댜댠댱더",
    "덕덖던덛덜덞덟덤덥덧덩덫덮덯데덱",
    "덴델뎀뎁뎃뎄뎅뎌뎐뎔뎠뎡뎨뎬도독",
    "돈돋돌돎돐돔돕돗동돛돝돠돤돨돼됏",
    "됐되된될됨됩됫됬됭됴두둑둔둗둘둚",
    "둠둡둣둥둬뒀뒈뒙뒝뒤뒨뒬뒵뒷뒸뒹",
    "듀듄듈듐듕드득든듣들듥듦듧듬듭듯",
    "등듸듼딀디딕딘딛딜딤딥딧딨딩딪딫",
    "딮따딱딲딴딷딸땀땁땃땄땅땋때땍땐",
    "땔땜땝땟땠땡떄떈떔떙떠떡떤떨떪떫",
    "떰떱떳떴떵떻떼떽뗀뗄뗌뗍뗏뗐뗑뗘",
    "뗬또똑똔똘똠똡똣똥똬똭똰똴뙇뙈뙜",
    "뙤뙨뚜뚝뚠뚤뚧뚫뚬뚯뚱뚸뛔뛰뛴뛸",
    "뜀뜁뜄뜅뜌뜨뜩뜬뜯뜰뜳뜸뜹뜻뜽뜾",
    "띃띄띈띌띔띕띠띡띤띨띰띱띳띵라락",
    "란랃랄람랍랏랐랑랒랖랗래랙랜랟랠",
    "램랩랫랬랭랰랲랴략랸럅럇량럐럔러",
];

/// Full details can be found in `masonry/resources/fonts/roboto_flex/README` from
/// the workspace root.
const HAHMLET_VARIABLE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/fonts/hahmlet/",
    "Hahmlet-VariableFont_wght.ttf",
));

fn run(event_loop: EventLoopBuilder) -> Result<(), EventLoopError> {
    let mut flex = Flex::column();
    for line in TEXT.iter().take(4) {
        let label = LoopingWeight::new(*line);
        flex = flex.with_child(label);
    }

    let window_attributes = Window::default_attributes().with_title("Simple image example");

    masonry::event_loop_runner::run(event_loop, window_attributes, RootWidget::new(flex), Driver)
}

struct LoopingWeight {
    child: WidgetPod<Label>,
    frame_index: u64,
}

impl LoopingWeight {
    fn new(text: impl Into<ArcStr>) -> Self {
        Self {
            child: WidgetPod::new(
                Label::new(text)
                    .with_style(StyleProperty::FontWeight(FontWeight::new(200.)))
                    .with_style(StyleProperty::FontStack(parley::style::FontStack::Source(
                        "Hahmlet".into(),
                    ))),
            ),
            frame_index: 0,
        }
    }
}

impl Widget for LoopingWeight {
    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx, _interval: u64) {
        let frame_number = (self.frame_index % 200) as f32;
        let weight = FontWeight::new(200. + 3. * frame_number);
        self.frame_index += 1;

        ctx.mutate_later(&mut self.child, move |mut child| {
            Label::insert_style(&mut child, StyleProperty::FontWeight(weight));
        });
        ctx.request_anim_frame();
    }
    fn register_children(&mut self, ctx: &mut RegisterCtx) {
        ctx.register_child(&mut self.child);
    }
    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = ctx.run_layout(&mut self.child, bc);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        size
    }
    fn paint(&mut self, _ctx: &mut PaintCtx, _scene: &mut Scene) {}
    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }
    fn update(&mut self, ctx: &mut UpdateCtx, event: &masonry::Update) {
        if let masonry::Update::WidgetAdded = event {
            ctx.request_anim_frame();
        }
    }
    fn accessibility(&mut self, _ctx: &mut AccessCtx, _node: &mut Node) {}
    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        [self.child.id()].as_slice().into()
    }
}

// Boilerplate code: Identical across all applications which support Android

#[expect(clippy::allow_attributes, reason = "No way to specify the condition")]
#[allow(dead_code, reason = "False positive: needed in not-_android version")]
// This is treated as dead code by the Android version of the example, but is actually live
// This hackery is required because Cargo doesn't care to support this use case, of one
// example which works across Android and desktop
fn main() -> Result<(), EventLoopError> {
    run(EventLoop::with_user_event())
}
#[cfg(target_os = "android")]
// Safety: We are following `android_activity`'s docs here
#[expect(
    unsafe_code,
    reason = "We believe that there are no other declarations using this name in the compiled objects here"
)]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut event_loop = EventLoop::with_user_event();
    event_loop.with_android_app(app);

    run(event_loop).expect("Can create app");
}
