use composure::rsx;

fn main() {
    let _ = rsx! {
        <Row>
            <Button custom_id="1">"1"</Button>
            <Button custom_id="2">"2"</Button>
            <Button custom_id="3">"3"</Button>
            <Button custom_id="4">"4"</Button>
            <Button custom_id="5">"5"</Button>
            <Button custom_id="6">"6"</Button>
        </Row>
        <Section>
            <Text>"1"</Text>
            <Text>"2"</Text>
            <Text>"3"</Text>
            <Text>"4"</Text>
            <Button custom_id="s">"S"</Button>
        </Section>
        <Row>
            <StringSelect custom_id="pick">
                <Option value="1">"1"</Option> <Option value="2">"2"</Option>
                <Option value="3">"3"</Option> <Option value="4">"4"</Option>
                <Option value="5">"5"</Option> <Option value="6">"6"</Option>
                <Option value="7">"7"</Option> <Option value="8">"8"</Option>
                <Option value="9">"9"</Option> <Option value="10">"10"</Option>
                <Option value="11">"11"</Option> <Option value="12">"12"</Option>
                <Option value="13">"13"</Option> <Option value="14">"14"</Option>
                <Option value="15">"15"</Option> <Option value="16">"16"</Option>
                <Option value="17">"17"</Option> <Option value="18">"18"</Option>
                <Option value="19">"19"</Option> <Option value="20">"20"</Option>
                <Option value="21">"21"</Option> <Option value="22">"22"</Option>
                <Option value="23">"23"</Option> <Option value="24">"24"</Option>
                <Option value="25">"25"</Option> <Option value="26">"26"</Option>
            </StringSelect>
        </Row>
    };
    let _ = rsx! {
        <Modal title="Too long" custom_id="form">
            "1" <Text>"2"</Text> <Text>"3"</Text> <Text>"4"</Text> <Text>"5"</Text> <Text>"6"</Text>
        </Modal>
    };
}
