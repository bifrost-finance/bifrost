use substrate_primitives::offchain::{HttpRequestStatus, Timestamp, HttpError};
use sr_io::offchain::http::Request;
use rstd::prelude::*;

pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/76.0.3809.132 Safari/537.36";

pub enum HttpFail {
	RequestStartFail,
	RequestAddHeaderFail,
	HttpError(HttpError),
	HttpRequestStatus(HttpRequestStatus),
}

pub fn http_request(
	uri: &str,
	method: &str,
	request_headers: Vec<(&str, &str)>,
	request_body: &[u8],
	timeout: Option<Timestamp>,
) -> Result<(Vec<(Vec<u8>, Vec<u8>)>, Vec<u8>), HttpFail> {
	let request_id = sr_io::http_request_start(method, uri, &[])
		.map_err(|_| HttpFail::RequestStartFail)?;

	for header in request_headers {
		let header_key = header.0;
		let header_value = header.1;
		sr_io::http_request_add_header(request_id, header_key, header_value)
			.map_err(|_| HttpFail::RequestAddHeaderFail)?;
	}

	sr_io::http_request_write_body(request_id, request_body, timeout)
		.map_err(|err| HttpFail::HttpError(err))?;

	let status_vec = sr_io::http_response_wait(&[request_id], timeout);

	#[cfg(feature = "std")]
	log::info!("status_vec: {:?}", status_vec);
	#[cfg(feature = "std")]
	log::info!("request_id: {:?}", request_id);
	match status_vec[0] {
		HttpRequestStatus::Finished(status_code) => {
			let response_headers = sr_io::http_response_headers(request_id);

			let mut response_body: Vec<u8> = Vec::new();
			loop {
				let mut buffer = [0u8; 1024];
				match sr_io::http_response_read_body(request_id, &mut buffer, timeout) {
					Ok(size) => {
						if size == 0 {
							return Ok((response_headers, response_body));
						}

						let data: &[u8] = &buffer[0..size];
						let msg: Vec<char> = data.into_iter().map(|d| *d as char).collect();
						#[cfg(feature = "std")]
							let msg: String = msg.into_iter().collect();
						#[cfg(feature = "std")]
						log::info!("response_body: {}", msg);

						let data: &[u8] = &buffer[0..size];
						let mut data: Vec<u8> = data.into_iter().map(|d| *d).collect();
						response_body.append(&mut data);
					}
					Err(err) => return Err(HttpFail::HttpError(err)),
				}
			}
		}
		_ => Err(HttpFail::HttpRequestStatus(status_vec[0]))
	}
}

pub fn make_request() {
	let method = "GET";
	let uri = "http://127.0.0.1:8080";
	let mut request_headers: Vec<(&str, &str)> = Vec::new();
	let request_body = &[];
	request_headers.push(("User-Agent", USER_AGENT));
	let ret = http_request(uri, method, request_headers, request_body, None);
	match ret {
		Ok((response_headers, response_body)) => {
			#[cfg(feature = "std")]
				let response_body: String = response_body.into_iter().map(|d| d as char).collect();
			#[cfg(feature = "std")]
			log::info!("response_headers: {:?}", response_headers);
			#[cfg(feature = "std")]
			log::info!("response_body: {:?}", response_body);
			log::info!("response_body length: {}", response_body.len());
		}
		Err(err) => {}
	}
}

pub fn make_request_2() -> Result<(), HttpError> {
	let uri = "http://127.0.0.1:8080";

	let request: Request = Request::get(uri);

	let pending = request
		.add_header("User-Agent", USER_AGENT)
		.send()?;

	// wait
	let mut response = pending
		.wait()
		.or(Err(HttpError::IoError))?;

	// then check the response
	let response_headers = response.headers();
	//	let headers = response.headers().into_iter();
	#[cfg(feature = "std")]
	log::info!("response_headers: {:?}", response_headers);

	let body = response.body();
	let body_data = body.clone().collect::<Vec<_>>();
	#[cfg(feature = "std")]
		let response_body: String = body_data.into_iter().map(|d| d as char).collect();
	#[cfg(feature = "std")]
	log::info!("response_body: {:?}", response_body);
	#[cfg(feature = "std")]
	log::info!("response_body length: {}", response_body.len());

	Ok(())
}

#[cfg(test)]
mod tests {
	use substrate_offchain::testing;
	use sr_io::offchain::http::Request;
	use sr_io::{TestExternalities, with_externalities};

	#[test]
	fn should_send_a_basic_request_and_get_response() {
		let (offchain, state) = testing::TestOffchainExt::new();
		let mut t = TestExternalities::default();
		t.set_offchain_externalities(offchain);

		with_externalities(&mut t, || {
			let request: Request = Request::get("http://127.0.0.1:8080/");
			let pending = request
				.send()
				.unwrap();
			// make sure it's sent correctly
			state.write().fulfill_pending_request(
				0,
				testing::PendingRequest {
					method: "GET".into(),
					uri: "http://127.0.0.1:8080/".into(),
					sent: true,
					..Default::default()
				},
				b"hello world".to_vec(),
				None,
			);

			// wait
			let mut response = pending.wait().unwrap();

			// then check the response
			let mut headers = response.headers().into_iter();
			assert_eq!(headers.current(), None);
			assert_eq!(headers.next(), false);
			assert_eq!(headers.current(), None);

			let body = response.body();
			assert_eq!(body.clone().collect::<Vec<_>>(), b"hello world".to_vec());
			assert_eq!(body.error(), &None);
		})
	}
}
