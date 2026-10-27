// Parse each line of the text into an expression tree and evaluate it, repeated.

enum Expr {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

fn parse_expr(toks: &[&str], pos: &mut usize) -> Expr {
    let mut e = parse_term(toks, pos);
    while *pos < toks.len() && (toks[*pos] == "+" || toks[*pos] == "-") {
        let op = toks[*pos];
        *pos += 1;
        let right = Box::new(parse_term(toks, pos));
        e = if op == "+" {
            Expr::Add(Box::new(e), right)
        } else {
            Expr::Sub(Box::new(e), right)
        };
    }
    e
}

fn parse_term(toks: &[&str], pos: &mut usize) -> Expr {
    let mut e = parse_factor(toks, pos);
    while *pos < toks.len() && toks[*pos] == "*" {
        *pos += 1;
        let right = Box::new(parse_factor(toks, pos));
        e = Expr::Mul(Box::new(e), right);
    }
    e
}

fn parse_factor(toks: &[&str], pos: &mut usize) -> Expr {
    *pos += 1;
    if toks[*pos - 1] == "(" {
        let e = parse_expr(toks, pos);
        *pos += 1;
        return e;
    }
    Expr::Num(toks[*pos - 1].parse().unwrap_or(0))
}

fn eval(e: &Expr) -> i64 {
    match e {
        Expr::Num(v) => *v,
        Expr::Add(l, r) => eval(l) + eval(r),
        Expr::Sub(l, r) => eval(l) - eval(r),
        Expr::Mul(l, r) => eval(l) * eval(r),
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: expr_parse <text>");
    let text = std::fs::read_to_string(path).expect("cannot read the text");
    let lines: Vec<&str> = text.lines().collect();
    let mut total: i64 = 0;
    for _ in 0..10 {
        for line in &lines {
            let toks: Vec<&str> = line.split_ascii_whitespace().collect();
            let e = parse_expr(&toks, &mut 0);
            total = total.wrapping_add(eval(&e));
        }
    }
    println!("{total}");
}
