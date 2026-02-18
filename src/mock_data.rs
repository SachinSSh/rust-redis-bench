use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use redis::aio::ConnectionManager;
use std::time::Instant;

// ─── Constants ───────────────────────────────────────────────────

const NUM_USERS: usize = 10_000;
const NUM_PRODUCTS: usize = 500;
/// Pipeline batch size — keeps Redis buffers comfortable.
const BATCH: usize = 500;

// ─── Name pools ──────────────────────────────────────────────────

static FIRST: &[&str] = &[
    "Emma",
    "Liam",
    "Olivia",
    "Noah",
    "Ava",
    "Ethan",
    "Sophia",
    "Mason",
    "Isabella",
    "William",
    "Mia",
    "James",
    "Charlotte",
    "Benjamin",
    "Amelia",
    "Lucas",
    "Harper",
    "Henry",
    "Evelyn",
    "Alexander",
    "Abigail",
    "Daniel",
    "Emily",
    "Michael",
    "Elizabeth",
    "Owen",
    "Sofia",
    "Sebastian",
    "Avery",
    "Jack",
];

static LAST: &[&str] = &[
    "Smith",
    "Johnson",
    "Williams",
    "Brown",
    "Jones",
    "Garcia",
    "Miller",
    "Davis",
    "Rodriguez",
    "Martinez",
    "Hernandez",
    "Lopez",
    "Gonzalez",
    "Wilson",
    "Anderson",
    "Thomas",
    "Taylor",
    "Moore",
    "Jackson",
    "Martin",
    "Lee",
    "Perez",
    "Thompson",
    "White",
    "Harris",
    "Sanchez",
    "Clark",
    "Ramirez",
    "Lewis",
    "Robinson",
];

static ROLES: &[&str] = &["admin", "editor", "viewer"];

static ADJ: &[&str] = &[
    "Premium",
    "Ultra",
    "Wireless",
    "Smart",
    "Compact",
    "Professional",
    "Ergonomic",
    "Portable",
    "Advanced",
    "Digital",
    "Classic",
    "Modern",
    "Elite",
    "Turbo",
    "Nano",
    "Dual",
    "Mini",
    "Pro",
    "Max",
    "Super",
];

static NOUN: &[&str] = &[
    "Keyboard",
    "Mouse",
    "Monitor",
    "Headphones",
    "Speaker",
    "Camera",
    "Microphone",
    "Tablet",
    "Charger",
    "Adapter",
    "Hub",
    "Cable",
    "Stand",
    "Light",
    "Webcam",
    "Router",
    "Drive",
    "Dock",
    "Controller",
    "Sensor",
];

static CAT: &[&str] = &[
    "electronics",
    "accessories",
    "audio",
    "computing",
    "peripherals",
    "networking",
    "storage",
    "gadgets",
    "office",
    "gaming",
];

// ─── Public entry point ──────────────────────────────────────────

pub async fn seed(conn: &ConnectionManager) {
    let start = Instant::now();
    println!(
        "Seeding {} users and {} products into Redis...",
        NUM_USERS, NUM_PRODUCTS
    );

    let mut conn = conn.clone();
    // Deterministic RNG so re-runs produce the same data.
    let mut rng = StdRng::seed_from_u64(42);

    seed_users(&mut conn, &mut rng).await;
    seed_products(&mut conn, &mut rng).await;

    println!(
        "   ✓ seed complete in {:.1}s",
        start.elapsed().as_secs_f64()
    );
}

// ─── Users ───────────────────────────────────────────────────────

async fn seed_users(conn: &mut ConnectionManager, rng: &mut StdRng) {
    for batch_start in (0..NUM_USERS).step_by(BATCH) {
        let batch_end = (batch_start + BATCH).min(NUM_USERS);
        let mut pipe = redis::pipe();

        for i in batch_start..batch_end {
            let id = format!("usr_{:08}", i + 1);
            let key = format!("user:{}", id);

            let first = FIRST[rng.gen_range(0..FIRST.len())];
            let last = LAST[rng.gen_range(0..LAST.len())];
            let name = format!("{} {}", first, last);
            let email = format!(
                "{}.{}{}@example.com",
                first.to_lowercase(),
                last.to_lowercase(),
                i + 1,
            );
            let role = ROLES[rng.gen_range(0..ROLES.len())];
            let theme = if rng.gen_bool(0.5) { "dark" } else { "light" };
            let notif = rng.gen_bool(0.7);
            let prefs = format!(
                r#"{{"theme":"{}","lang":"en","notifications":{}}}"#,
                theme, notif,
            );
            let created = "2025-01-15T09:23:11Z";

            pipe.cmd("HSET")
                .arg(&key)
                .arg("id")
                .arg(&id)
                .arg("name")
                .arg(&name)
                .arg("email")
                .arg(&email)
                .arg("role")
                .arg(role)
                .arg("prefs")
                .arg(&prefs)
                .arg("created_at")
                .arg(created)
                .ignore();
        }

        let _: () = pipe.query_async(conn).await.expect("Failed to seed users");
    }
}

// ─── Products ────────────────────────────────────────────────────

async fn seed_products(conn: &mut ConnectionManager, rng: &mut StdRng) {
    let mut pipe = redis::pipe();

    for i in 0..NUM_PRODUCTS {
        let id = format!("prod_{:04}", i + 1);
        let key = format!("product:{}", id);

        let adj = ADJ[rng.gen_range(0..ADJ.len())];
        let noun = NOUN[rng.gen_range(0..NOUN.len())];
        let title = format!("{} {}", adj, noun);
        let category = CAT[rng.gen_range(0..CAT.len())];
        let price = rng.gen_range(999..=99_999u64); // cents
        let stock = rng.gen_range(0..=1000u32);
        let desc = format!(
            "High-quality {} {} with advanced features. \
             Perfect for {} use. Built with premium materials \
             for long-lasting durability and peak performance.",
            adj.to_lowercase(),
            noun.to_lowercase(),
            category,
        );

        pipe.cmd("HSET")
            .arg(&key)
            .arg("id")
            .arg(&id)
            .arg("title")
            .arg(&title)
            .arg("price")
            .arg(price)
            .arg("stock")
            .arg(stock)
            .arg("category")
            .arg(category)
            .arg("description")
            .arg(&desc)
            .ignore();
    }

    let _: () = pipe
        .query_async(conn)
        .await
        .expect("Failed to seed products");
}
