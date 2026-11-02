//! The small integer expression language shared by assembler equates and
//! immediates and by the debugger's `print`, `x`, and `set` commands:
//! integers, characters, symbols, parentheses, unary minus, and `+ - * / %`
//! with the usual precedence. Callers decide what a symbol token means.

/// Evaluates `text`, resolving each non-literal token through `resolve`.
/// Tokens are `[$&]?[A-Za-z0-9_.]+` or a quoted character such as `'0'`.
pub(crate) fn evaluate(
    text: &str,
    resolve: impl Fn(&str) -> Result<i128, String>,
) -> Result<i128, String> {
    let mut parser = Parser {
        text,
        position: 0,
        resolve: &resolve,
    };
    let value = parser.sum()?;
    parser.skip_whitespace();
    if parser.position != text.len() {
        return Err(format!(
            "unexpected `{}` in expression `{text}`",
            &text[parser.position..]
        ));
    }
    Ok(value)
}

/// Converts an evaluated value to the 64-bit two's-complement range that a
/// literal immediate may occupy.
pub(crate) fn to_u64(value: i128, text: &str) -> Result<u64, String> {
    if value < i128::from(i64::MIN) || value > i128::from(u64::MAX) {
        return Err(format!("expression `{text}` does not fit in 64 bits"));
    }
    Ok(value as u64)
}

struct Parser<'a> {
    text: &'a str,
    position: usize,
    resolve: &'a dyn Fn(&str) -> Result<i128, String>,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self.text[self.position..].starts_with(char::is_whitespace) {
            self.position += 1;
        }
    }

    fn eat(&mut self, character: char) -> bool {
        self.skip_whitespace();
        if self.text[self.position..].starts_with(character) {
            self.position += character.len_utf8();
            true
        } else {
            false
        }
    }

    fn sum(&mut self) -> Result<i128, String> {
        let mut value = self.product()?;
        loop {
            if self.eat('+') {
                value = value
                    .checked_add(self.product()?)
                    .ok_or("expression overflows")?;
            } else if self.eat('-') {
                value = value
                    .checked_sub(self.product()?)
                    .ok_or("expression overflows")?;
            } else {
                return Ok(value);
            }
        }
    }

    fn product(&mut self) -> Result<i128, String> {
        let mut value = self.unary()?;
        loop {
            if self.eat('*') {
                value = value
                    .checked_mul(self.unary()?)
                    .ok_or("expression overflows")?;
            } else if self.eat('/') {
                let divisor = self.unary()?;
                value = value
                    .checked_div(divisor)
                    .ok_or_else(|| format!("division by zero in `{}`", self.text))?;
            } else if self.eat('%') {
                let divisor = self.unary()?;
                value = value
                    .checked_rem(divisor)
                    .ok_or_else(|| format!("division by zero in `{}`", self.text))?;
            } else {
                return Ok(value);
            }
        }
    }

    fn unary(&mut self) -> Result<i128, String> {
        if self.eat('-') {
            return Ok(-self.unary()?);
        }
        if self.eat('+') {
            return self.unary();
        }
        if self.eat('(') {
            let value = self.sum()?;
            if !self.eat(')') {
                return Err(format!("missing `)` in expression `{}`", self.text));
            }
            return Ok(value);
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<i128, String> {
        self.skip_whitespace();
        let rest = &self.text[self.position..];
        let length = if let Some(character) = rest.strip_prefix('\'') {
            character
                .find('\'')
                .map(|end| end + 2)
                .ok_or_else(|| format!("unterminated character in `{}`", self.text))?
        } else {
            let prefix = usize::from(rest.starts_with(['$', '&']));
            prefix
                + rest[prefix..]
                    .find(|character: char| {
                        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
                    })
                    .unwrap_or(rest.len() - prefix)
        };
        if length == 0 || (length == 1 && rest.starts_with(['$', '&'])) {
            return Err(format!("expected a value in expression `{}`", self.text));
        }
        let token = &rest[..length];
        self.position += length;
        (self.resolve)(token)
    }
}
