use composure::rsx;

fn main() {
    let count: u32 = 1;
    let _ = rsx! {
        <Text>{count.missing()}</Text>
        <Container accent_color=0x5865F2>
            <Text>"Never closed"</Text>
    };
}
